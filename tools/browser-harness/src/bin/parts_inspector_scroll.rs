//! Manual Parts inspector scroll regression against a running site.
#[path = "../shutdown.rs"]
mod shutdown;

use boardstudio_browser_harness::{
    driver::{self, HandlerErrors, PageErrors},
    Error, IsolatedDirs, Result,
};
use chromiumoxide::{
    cdp::browser_protocol::{
        emulation::SetDeviceMetricsOverrideParams,
        input::{DispatchMouseEventParams, DispatchMouseEventType},
    },
    Page,
};
use std::{path::Path, sync::Arc, time::Duration};

const WAIT: Duration = Duration::from_secs(30);
fn err(e: impl std::fmt::Display) -> Error {
    Error(e.to_string())
}

async fn scenario(page: &Page, width: i64, height: i64, url: &str) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(60), page.goto(url))
        .await
        .map_err(err)?
        .map_err(err)?;
    for name in [
        "Create new keyboard",
        "Split keyboard",
        "Continue to layout",
        "Create matrix",
    ] {
        driver::click_role(page, None, "button", name).await?;
    }
    driver::backend_of_wait(
        page,
        ".m1-scene-part[data-part-id^='matrix/']",
        Duration::from_secs(60),
    )
    .await?;
    driver::click_role(page, None, "button", "Hide guide").await?;
    driver::click_role(page, None, "button", "Open Parts workspace").await?;
    let root = driver::backend_of_wait(page, "#m1-objects-panel", WAIT).await?;
    driver::click_role(page, Some(root), "button", "New assembly").await?;
    driver::click_role(page, None, "option", "MX Hotswap").await?;
    page.execute(SetDeviceMetricsOverrideParams::new(
        width, height, 1.0, false,
    ))
    .await
    .map_err(err)?;
    driver::wait_for(
        page,
        &format!("innerWidth === {width} && innerHeight === {height}"),
        "viewport",
        WAIT,
    )
    .await?;
    let point = driver::eval(page, "(() => { const b=document.querySelector('.m1-inspector-body').getBoundingClientRect(); document.querySelector('.m1-inspector-body').addEventListener('wheel', e => window.__partsWheelTrusted=e.isTrusted, {once:true}); return {x:b.x+b.width/2,y:Math.min(b.y+b.height/2,innerHeight-40)} })()").await?;
    let x = point["x"]
        .as_f64()
        .ok_or_else(|| err("missing inspector x"))?;
    let y = point["y"]
        .as_f64()
        .ok_or_else(|| err("missing inspector y"))?;
    for kind in [
        DispatchMouseEventType::MouseMoved,
        DispatchMouseEventType::MouseWheel,
    ] {
        let mut event = DispatchMouseEventParams::builder()
            .r#type(kind.clone())
            .x(x)
            .y(y);
        if matches!(kind, DispatchMouseEventType::MouseWheel) {
            event = event.delta_x(0.0).delta_y(1000.0);
        }
        page.execute(event.build().map_err(err)?)
            .await
            .map_err(err)?;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    let state = driver::eval(page, "(() => { const e=document.querySelector('[aria-label=side]'), r=e.getBoundingClientRect(), body=e.closest('.m1-inspector-body'); return {top:r.top,bottom:r.bottom,scrollTop:body.scrollTop,overflow:getComputedStyle(body).overflowY,hit:r.top>=0 && r.bottom<=innerHeight && document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)===e,pageOverflow:document.documentElement.scrollWidth>innerWidth,trusted:window.__partsWheelTrusted===true} })()").await?;
    println!("{width}x{height} {state}");
    if state["scrollTop"].as_f64().unwrap_or(0.0) <= 0.0
        || state["hit"] != true
        || state["pageOverflow"] != false
        || state["trusted"] != true
    {
        return Err(err(
            "wheel must expose the board side selector without horizontal overflow",
        ));
    }
    let side = driver::backend_of(page, "[aria-label=side]").await?;
    driver::click_actionable(page, side, Duration::from_secs(5)).await?;
    driver::press_key(page, "End", "End", 35).await?;
    driver::press_key(page, "Enter", "Enter", 13).await?;
    if driver::eval(page, "document.querySelector('[aria-label=side]').value").await? != "B" {
        return Err(err("board side must change via pointer and keyboard"));
    }
    println!("PASS {width}x{height}: board side accessible and changed");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = main_run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn main_run() -> Result<()> {
    let url =
        std::env::var("BOARDSTUDIO_PREVIEW_URL").map_err(|_| err("set BOARDSTUDIO_PREVIEW_URL"))?;
    let chrome =
        std::env::var("CHROME_BIN").map_err(|_| err("set CHROME_BIN to Chrome or Chromium"))?;
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let dirs = IsolatedDirs::create(Path::new(&tmp)).map_err(err)?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let he = Arc::new(HandlerErrors::new());
    let pe = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(handler, Arc::clone(&he), Arc::clone(&pe));
    let outcome = async {
        for (width, height) in [(390, 844), (1513, 1043)] {
            let (context, page) = tokio::time::timeout(
                Duration::from_secs(60),
                driver::new_isolated_page(&browser, "about:blank", &pe),
            )
            .await
            .map_err(err)??;
            let result = scenario(&page, width, height, &url).await;
            let disposal = driver::dispose_isolated_page(&browser, Some(context), page, &pe).await;
            match (result, disposal) {
                (Err(a), Err(b)) => return Err(err(format!("{a}; {b}"))),
                (Err(e), _) | (_, Err(e)) => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }
    .await;
    let (shutdown, confirmed) =
        shutdown::finish_browser(browser, drain, &he, outcome.is_ok()).await;
    let cleanup = if confirmed {
        shutdown::remove_profile(&dirs)
    } else {
        eprintln!(
            "browser termination unconfirmed; preserving profile at {}",
            dirs.profile_dir.display()
        );
        std::mem::forget(dirs);
        Ok(())
    };
    let errors = [
        outcome.err(),
        shutdown.err(),
        pe.fail_if_any().err(),
        cleanup.err(),
    ]
    .into_iter()
    .flatten()
    .map(|e| e.to_string())
    .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(err(errors.join("; ")))
    }
}
