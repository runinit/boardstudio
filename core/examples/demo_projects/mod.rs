mod physical;
mod recipes;
mod review;
use boardstudio_core::{CoreEngine, archive, generators, model::ProjectDoc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
fn read(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(root().join(path))?)?)
}
fn s(v: &Value) -> &str {
    v.as_str().expect("recipe text")
}
fn n(v: &Value) -> f64 {
    v.as_f64().expect("recipe number")
}
fn a(v: &Value) -> &Vec<Value> {
    v.as_array().expect("recipe list")
}
fn push(v: &mut Value, key: &str, item: Value) {
    v[key].as_array_mut().unwrap().push(item);
}
fn empty(id: &str, name: &str) -> Value {
    serde_json::to_value(ProjectDoc::empty(id, name)).unwrap()
}
fn outline(id: &str, ids: Value) -> Value {
    json!({"id":id,"kind":"part-envelope","operation":"add","partIds":ids,"margin":4,"settings":{"corners":"fillet","size":2,"bridgeWidth":10}})
}
fn definition(doc: &mut Value, source: &str, id: &str, params: Value) -> Result<String> {
    let mut d = generators::catalogue()?
        .into_iter()
        .find(|d| {
            d.generator
                .as_ref()
                .is_some_and(|g| g.source == format!("ceoloide/{source}"))
        })
        .ok_or("Missing generator")?;
    d.id = id.into();
    d.generator.as_mut().unwrap().parameters = serde_json::from_value(params)?;
    push(
        doc,
        "definitions",
        serde_json::to_value(generators::normalize_definition(d)?)?,
    );
    Ok(id.into())
}
fn request(engine: &mut CoreEngine, input: Value) -> Result<Value> {
    let reply: Value = serde_json::from_str(&engine.request(&input.to_string()))?;
    if reply["kind"] == "error" {
        return Err(s(&reply["message"]).to_owned().into());
    }
    Ok(reply)
}
fn edit(engine: &mut CoreEngine, doc: Value, operation: Value, transaction: &str) -> Result<Value> {
    Ok(request(engine,json!({"id":transaction,"kind":"edit","command":{"baseRevision":doc["revision"],"transactionId":transaction,"phase":"commit","targetIds":[],"operation":operation}}))?["document"].take())
}
fn open(engine: &mut CoreEngine, doc: Value) -> Result<Value> {
    let reply = request(engine, json!({"id":"prepare","kind":"open","document":doc}))?;
    if reply["kind"] != "scene" {
        return Err("Expected scene".into());
    }
    Ok(reply["document"].clone())
}
fn wire(engine: &mut CoreEngine, mut doc: Value) -> Result<Value> {
    for board in a(&doc["boards"]).clone() {
        let id = s(&board["id"]);
        let resolved = request(
            engine,
            json!({"id":"wire","kind":"resolve-electrical","request":{"document":doc,"instanceId":id,"boardId":id,"controllerPartId":format!("{id}/U1"),"controllerProfile":null,"mode":"matrix","locks":{}}}),
        )?;
        if resolved["kind"] != "electrical-resolved" {
            return Err("Expected electrical plan".into());
        }
        for d in a(&resolved["plan"]["diagnostics"]) {
            if d["severity"] == "error" {
                return Err(s(&d["message"]).to_owned().into());
            }
        }
        doc=request(engine,json!({"id":"apply","kind":"apply-electrical","baseRevision":doc["revision"],"plan":resolved["plan"],"draft":false}))?["document"].take();
    }
    Ok(doc)
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn archive_request(input: Value, buffers: &[Vec<u8>]) -> Result<(Value, Vec<Vec<u8>>)> {
    let (reply, buffers) = archive::request(&input.to_string(), buffers);
    let reply: Value = serde_json::from_str(&reply)?;
    if reply["kind"] == "error" {
        return Err(s(&reply["message"]).to_owned().into());
    }
    Ok((reply, buffers))
}
fn model_path(id: &str) -> Result<PathBuf> {
    let path = id
        .strip_prefix("bundled-model:")
        .ok_or("Not a bundled model")?;
    let (vendor, name) = path.split_once('/').ok_or("Invalid model id")?;
    let name = match (vendor, name) {
        ("thqwgd001", "THQWGD001 #1.stp") => "THQWGD001-rotation.stp",
        ("thqwgd001", "THQWGD001C [2pin] #1.stp") => "THQWGD001C-2pin.stp",
        ("thqwgd001", "THQWGD001C [4pin] #1.stp") => "THQWGD001C-4pin.stp",
        _ => name,
    };
    let base = root().join("ergogen/library/vendor").canonicalize()?;
    let file = base
        .join(vendor)
        .join("3d_models")
        .join(name)
        .canonicalize()?;
    if !file.starts_with(&base) {
        return Err("Model outside vendor directory".into());
    }
    Ok(file)
}
fn pack(mut doc: Value, output: &Path, name: &str) -> Result<Value> {
    let typed: ProjectDoc = serde_json::from_value(doc.clone())?;
    let mut ids = BTreeSet::new();
    for d in &typed.definitions {
        if d.generator.is_some() {
            for p in typed.parts.iter().filter(|p| p.definition_id == d.id) {
                ids.extend(generators::model_asset_ids(d, Some(p))?);
            }
        }
    }
    fn collect(v: &Value, ids: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(models) = o.get("models").and_then(Value::as_array) {
                    ids.extend(
                        models
                            .iter()
                            .filter_map(|m| m.get("assetId").and_then(Value::as_str))
                            .filter(|id| id.starts_with("bundled-model:"))
                            .map(str::to_owned),
                    );
                }
                for item in o.values() {
                    collect(item, ids);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect(item, ids);
                }
            }
            _ => {}
        }
    }
    collect(&doc, &mut ids);
    for assembly in &typed.assemblies {
        for member in &assembly.members {
            if let Some(d) = typed
                .definitions
                .iter()
                .find(|d| Some(&d.id) == member.definition_id.as_ref())
                .filter(|d| d.generator.is_some())
            {
                let mut d = d.clone();
                d.generator
                    .as_mut()
                    .unwrap()
                    .parameters
                    .extend(member.parameters.clone().unwrap_or_default());
                let p = serde_json::from_value(
                    json!({"id":member.id,"definitionId":d.id,"reference":member.id,"pose":member.pose,"side":member.side}),
                )?;
                ids.extend(generators::model_asset_ids(&d, Some(&p))?);
            }
        }
    }
    for reference in &typed.board_references {
        ids.extend(
            reference
                .model_assets
                .values()
                .filter(|id| id.starts_with("bundled-model:"))
                .cloned(),
        );
    }
    if !typed.assets.is_empty() {
        return Err("Generated recipe unexpectedly carries local assets".into());
    }
    let mut files = BTreeMap::new();
    let mut assets = Vec::new();
    for id in ids {
        let bytes = fs::read(model_path(&id)?)?;
        let hash = digest(&bytes);
        let filename = id
            .strip_prefix("bundled-model:")
            .unwrap()
            .split_once('/')
            .unwrap()
            .1;
        let lower = filename.to_lowercase();
        let media = if lower.ends_with(".wrl") {
            "model/vrml"
        } else if lower.ends_with(".stl") {
            "model/stl"
        } else {
            "model/step"
        };
        push(
            &mut doc,
            "assets",
            json!({"id":id,"name":filename,"mediaType":media,"sha256":hash,"source":"bundled footprint library"}),
        );
        files.insert(hash, bytes);
    }
    let mut buffers = Vec::new();
    let mut entries = Vec::new();
    for (hash, bytes) in files {
        fs::write(output.join(&hash), &bytes)?;
        assets.push(json!({"sha256":hash,"bytes":bytes.len()}));
        entries.push(json!({"path":format!("assets/{hash}"),"bufferIndex":buffers.len()}));
        buffers.push(bytes);
    }
    let text = doc.to_string();
    let (_, packed) = archive_request(
        json!({"kind":"pack-project","projectJson":text,"archiveJson":"{\"embedUsedModels\":true}","assets":entries}),
        &buffers,
    )?;
    let bytes = &packed[0];
    fs::write(output.join(format!("{name}.json")), text)?;
    fs::write(output.join(format!("{name}.boardstudio")), bytes)?;
    Ok(
        json!({"name":doc["name"],"fixture":name,"document_id":doc["id"],"revision":doc["revision"],"archive_sha256":digest(bytes),"assets":assets,"boards":doc["boards"],"key_count":typed.matrices.iter().flat_map(|m|&m.cells).filter(|c|c.enabled).count(),"modules":doc["modules"]}),
    )
}
pub fn prepare(destination: Option<&str>) -> Result<()> {
    let output = destination
        .map(PathBuf::from)
        .unwrap_or_else(|| root().join("web/assets/fixtures"));
    if output.exists() {
        let provenance: Value = serde_json::from_slice(
            &fs::read(output.join("provenance.json"))
                .map_err(|_| "Destination is not a prepared fixture directory")?,
        )?;
        if provenance["schema"] != 1 || !provenance["fixtures"].is_array() {
            return Err("Destination is not a prepared fixture directory".into());
        }
    }
    let parent = output.parent().ok_or("Output needs a parent")?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".demo-projects-{}", std::process::id()));
    fs::create_dir(&staging)?;
    let result = (|| -> Result<()> {
        let mut fixtures = Vec::new();
        let mut engine = CoreEngine::new();
        let bytes = fs::read(root().join("content/archives/reviung41-original.boardstudio"))?;
        let (reply, buffers) = archive_request(
            json!({"kind":"unpack-project"}),
            std::slice::from_ref(&bytes),
        )?;
        let mut assets = Vec::new();
        for asset in a(&reply["assets"]) {
            let b = &buffers[asset["bufferIndex"].as_u64().unwrap() as usize];
            let hash = s(&asset["sha256"]);
            fs::write(staging.join(hash), b)?;
            assets.push(json!({"sha256":hash,"bytes":b.len()}));
        }
        fs::write(staging.join("reviung41.json"), s(&reply["projectJson"]))?;
        fs::write(staging.join("reviung41.boardstudio"), &bytes)?;
        let doc: Value = serde_json::from_str(s(&reply["projectJson"]))?;
        fixtures.push(json!({"name":"REVIUNG41","source":"content/archives/reviung41-original.boardstudio","source_sha256":digest(&bytes),"document_id":doc["id"],"revision":doc["revision"],"assets":assets}));
        for variant in ["v2", "rgb", "choc"] {
            let seed = recipes::sofle(variant)?;
            let opened = open(&mut engine, seed)?;
            let mut doc = wire(&mut engine, opened)?;
            if variant == "v2" {
                doc = edit(
                    &mut engine,
                    doc,
                    json!({"kind":"set-mechanical","configuration":recipes::gasket()}),
                    "m1-fixture-gasket",
                )?;
            }
            fixtures.push(pack(
                doc,
                &staging,
                &if variant == "v2" {
                    "sofle".into()
                } else {
                    format!("sofle-{variant}")
                },
            )?);
        }
        let layouts = read("content/layouts/keyboard-layouts.json")?;
        for (id, layout) in layouts.as_object().unwrap() {
            let seed = recipes::keyboard(id, layout)?;
            let opened = open(&mut engine, seed)?;
            let doc = wire(&mut engine, opened)?;
            fixtures.push(pack(doc, &staging, &format!("measured-{id}"))?);
        }
        let doc = review::build(&mut engine)?;
        fixtures.push(pack(doc, &staging, "vik-module-review")?);
        let mut hashes = BTreeMap::new();
        for file in [
            "core/examples/prepare_demo_projects.rs",
            "core/examples/demo_projects/mod.rs",
            "core/examples/demo_projects/physical.rs",
            "core/examples/demo_projects/recipes.rs",
            "core/examples/demo_projects/review.rs",
            "content/layouts/keyboard-layouts.json",
            "content/layouts/sofle-layouts.json",
            "content/layouts/module-review.json",
            "catalogue/modules/imported-modules.json",
            "catalogue/parts/imported-parts.json",
            "core/Cargo.toml",
            "Cargo.lock",
        ] {
            hashes.insert(file, digest(&fs::read(root().join(file))?));
        }
        let commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root())
            .output()?;
        fs::write(
            staging.join("provenance.json"),
            serde_json::to_vec_pretty(
                &json!({"schema":1,"source_commit":String::from_utf8(commit.stdout)?.trim(),"source_hashes":hashes,"preparation":"Native Rust recipes, Core normalization, public electrical resolution/application and edit operations, Core archives with embedded used models","fixtures":fixtures}),
            )?,
        )?;
        Ok(())
    })();
    if let Err(error) = result {
        fs::remove_dir_all(&staging)?;
        return Err(error);
    }
    if output.exists() {
        fs::remove_dir_all(&output)?;
    }
    fs::rename(staging, &output)?;
    println!("Prepared fixtures in {}", output.display());
    Ok(())
}
