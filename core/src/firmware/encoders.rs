use super::{FirmwareRequest, gpio_spec_flags};
use crate::model::{EncoderDriver, RotaryProfile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FirmwareEncoder {
    pub id: String,
    pub profile: RotaryProfile,
    pub a_gpio: String,
    pub b_gpio: String,
}

fn label(id: &str) -> String {
    format!(
        "encoder_{}",
        id.bytes().map(|b| format!("{b:02x}")).collect::<String>()
    )
}

pub(super) fn prepare(request: &mut FirmwareRequest) -> Result<(), String> {
    let all = request
        .encoders
        .iter()
        .chain(request.peripheral.iter().flat_map(|p| p.encoders.iter()))
        .cloned()
        .collect::<Vec<_>>();
    if all.is_empty() {
        return Ok(());
    }
    let ids = request
        .encoder_ids
        .iter()
        .chain(request.peripheral.iter().flat_map(|p| p.encoder_ids.iter()))
        .cloned()
        .collect::<Vec<_>>();
    let mut unique = BTreeSet::new();
    if ids.len() > 128
        || ids
            .iter()
            .any(|id| id.is_empty() || id.len() > 128 || !unique.insert(id))
    {
        return Err("Encoder sensor identities must be unique and bounded".into());
    }
    let sensor_settings = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let encoder = all
                .iter()
                .find(|e| &e.id == id)
                .ok_or_else(|| format!("Missing rotary profile for {id}"))?;
            let triggers = encoder
                .profile
                .triggers_per_rotation
                .filter(|n| *n > 0)
                .ok_or_else(|| format!("Encoder {id} needs verified triggers per rotation"))?;
            Ok(format!(
                "sensor_{i} {{ triggers-per-rotation = <{triggers}>; }};"
            ))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join(" ");
    fn half(request: &mut FirmwareRequest, ids: &[String], settings: &str) -> Result<(), String> {
        let controller = crate::electrical_profiles::profile(&request.controller_profile)
            .ok_or("Unsupported encoder controller")?;
        let mut used = request
            .rows
            .iter()
            .chain(&request.columns)
            .chain(&request.direct_pins)
            .chain(&request.auxiliary_pins)
            .chain(request.uart_tx.iter())
            .chain(request.uart_rx.iter())
            .map(|p| p.gpio.clone())
            .collect::<BTreeSet<_>>();
        for encoder in &request.encoders {
            if !ids.contains(&encoder.id) {
                return Err(format!(
                    "Encoder {} is missing from sensor order",
                    encoder.id
                ));
            }
            if encoder.profile.driver != Some(EncoderDriver::Ec11) {
                return Err(format!(
                    "Encoder {} needs a verified firmware driver",
                    encoder.id
                ));
            }
            let steps = encoder.profile.steps.filter(|n| *n > 0).ok_or_else(|| {
                format!("Encoder {} needs verified steps per rotation", encoder.id)
            })?;
            for gpio in [&encoder.a_gpio, &encoder.b_gpio] {
                if !controller.pins.iter().any(|p| p.firmware_gpio == gpio)
                    || controller.reserved_gpios.contains(&gpio.as_str())
                    || !used.insert(gpio.clone())
                {
                    return Err(format!(
                        "Encoder {} GPIO {gpio} is unavailable or conflicts",
                        encoder.id
                    ));
                }
            }
            let a = gpio_spec_flags(&encoder.a_gpio, "GPIO_ACTIVE_HIGH | GPIO_PULL_UP");
            let b = gpio_spec_flags(&encoder.b_gpio, "GPIO_ACTIVE_HIGH | GPIO_PULL_UP");
            let name = label(&encoder.id);
            request.peripheral_overlays.push(format!("/ {{ {name}: {name} {{ compatible = \"alps,ec11\"; status = \"okay\"; a-gpios = <{a}>; b-gpios = <{b}>; steps = <{steps}>; }}; }};"));
        }
        for id in ids
            .iter()
            .filter(|id| !request.encoders.iter().any(|e| &e.id == *id))
        {
            let name = label(id);
            request.peripheral_overlays.push(format!(
                "/ {{ {name}: {name} {{ compatible = \"alps,ec11\"; status = \"disabled\"; }}; }};"
            ));
        }
        request.peripheral_overlays.push(format!("/ {{ sensors: sensors {{ compatible = \"zmk,keymap-sensors\"; sensors = <{}>; {settings} }}; }};",ids.iter().map(|id| format!("&{}",label(id))).collect::<Vec<_>>().join(" ")));
        if !request.encoders.is_empty() {
            request.peripheral_config.extend([
                "CONFIG_EC11=y".into(),
                "CONFIG_EC11_TRIGGER_GLOBAL_THREAD=y".into(),
            ]);
        }
        Ok(())
    }
    half(request, &ids, &sensor_settings)?;
    if let Some(peripheral) = &mut request.peripheral {
        half(peripheral, &ids, &sensor_settings)?;
    }
    Ok(())
}
