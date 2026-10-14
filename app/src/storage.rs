//! Local-first persistence: projects live in IndexedDB in this browser
//! and in files the user saves. Nothing is uploaded anywhere.

use crate::js;
use easypro_core::project::Project;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub key: String,
    pub name: String,
    pub modified: String,
    #[serde(default)]
    pub parts: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub label: String,
    pub time: String,
    pub project: String,
}

async fn wait(p: js_sys::Promise) -> Result<JsValue, String> {
    JsFuture::from(p)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
}

pub async fn save_local(key: &str, project: &Project) -> Result<(), String> {
    let json = project.to_json();
    wait(js::idb_put("projects", key, &json)).await?;
    let parts = project
        .boards
        .iter()
        .map(|b| b.schematic.components().count())
        .sum();
    let meta = ProjectMeta {
        key: key.into(),
        name: project.name.clone(),
        modified: js::now_iso(),
        parts,
    };
    wait(js::idb_put(
        "meta",
        key,
        &serde_json::to_string(&meta).unwrap(),
    ))
    .await?;
    Ok(())
}

pub async fn load_local(key: &str) -> Result<Project, String> {
    let v = wait(js::idb_get("projects", key)).await?;
    let text = v.as_string().ok_or("project not found in this browser")?;
    Project::from_json(&text).map_err(|e| e.to_string())
}

pub async fn delete_local(key: &str) -> Result<(), String> {
    wait(js::idb_delete("projects", key)).await?;
    wait(js::idb_delete("meta", key)).await?;
    for (k, _) in all("snapshots").await? {
        if k.starts_with(&format!("{key}|")) {
            wait(js::idb_delete("snapshots", &k)).await?;
        }
    }
    Ok(())
}

async fn all(store: &str) -> Result<Vec<(String, String)>, String> {
    let v = wait(js::idb_all(store)).await?;
    let arr: js_sys::Array = v.into();
    Ok(arr
        .iter()
        .filter_map(|pair| {
            let pair: js_sys::Array = pair.into();
            Some((pair.get(0).as_string()?, pair.get(1).as_string()?))
        })
        .collect())
}

/// Recent projects, newest first.
pub async fn list_local() -> Vec<ProjectMeta> {
    let mut v: Vec<ProjectMeta> = all("meta")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(_, j)| serde_json::from_str(&j).ok())
        .collect();
    v.sort_by(|a, b| b.modified.cmp(&a.modified));
    v
}

/// Store a named version of the project ("version history").
pub async fn snapshot(key: &str, project: &Project, label: &str) -> Result<(), String> {
    let time = js::now_iso();
    let s = Snapshot {
        label: label.into(),
        time: time.clone(),
        project: project.to_json(),
    };
    wait(js::idb_put(
        "snapshots",
        &format!("{key}|{time}"),
        &serde_json::to_string(&s).unwrap(),
    ))
    .await?;
    Ok(())
}

pub async fn snapshots(key: &str) -> Vec<(String, Snapshot)> {
    let mut v: Vec<(String, Snapshot)> = all("snapshots")
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|(k, _)| k.starts_with(&format!("{key}|")))
        .filter_map(|(k, j)| Some((k, serde_json::from_str(&j).ok()?)))
        .collect();
    v.sort_by(|a, b| b.1.time.cmp(&a.1.time));
    v
}

pub async fn delete_snapshot(id: &str) -> Result<(), String> {
    wait(js::idb_delete("snapshots", id)).await.map(|_| ())
}

/// Ask for a file; returns (name, text).
pub async fn pick_file(accept: &str) -> Option<(String, String)> {
    let v = wait(js::pick_file(accept)).await.ok()?;
    if v.is_null() || v.is_undefined() {
        return None;
    }
    let arr: js_sys::Array = v.into();
    Some((arr.get(0).as_string()?, arr.get(1).as_string()?))
}

/// Save text through the File System Access API (re-using the handle on
/// later saves) or as a download. Returns the file name, or None if the
/// user cancelled.
pub async fn save_text(key: &str, name: &str, text: &str) -> Option<String> {
    wait(js::save_text(key, name, text)).await.ok()?.as_string()
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn get_pref(key: &str) -> Option<String> {
    local_storage()?.get_item(key).ok()?
}

pub fn set_pref(key: &str, value: &str) {
    if let Some(s) = local_storage() {
        let _ = s.set_item(key, value);
    }
}

const RECENT_PARTS: &str = "easypro.recent_parts";

/// Device ids placed most recently, newest first.
pub fn recent_parts() -> Vec<String> {
    get_pref(RECENT_PARTS)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn remember_part(id: &str) {
    let mut v = recent_parts();
    v.retain(|x| x != id);
    v.insert(0, id.to_string());
    v.truncate(8);
    set_pref(RECENT_PARTS, &serde_json::to_string(&v).unwrap());
}

/// Random-enough key for a new project.
pub fn new_key() -> String {
    format!(
        "p{:x}{:06x}",
        js_sys::Date::now() as u64,
        (js_sys::Math::random() * 16_777_215.0) as u32
    )
}
