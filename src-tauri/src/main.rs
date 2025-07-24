// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Serialize, Deserialize};
use std::{fs, sync::Mutex};
use tauri::{State, Window, Manager, Emitter, AppHandle};

// Plugins
use tauri_plugin_updater::Builder as UpdaterBuilder;
use tauri_plugin_dialog::init as dialog_init;
use tauri_plugin_process::init as process_init;

#[derive(Serialize, Deserialize, Clone)]
struct Note { id: u32, content: String, pinned: bool }

#[derive(Default)]
struct AppState {
  notes: Mutex<Vec<Note>>,
  next_id: Mutex<u32>,
}

// 📍 Créer un chemin vers notes.json via AppHandle
fn notes_file_path(handle: &AppHandle) -> std::path::PathBuf {
  let mut dir = handle
    .path()
    .app_data_dir()
    .expect("Impossible de récupérer app_data_dir");
  dir.push("notes.json");
  dir
}


// 📥 Charger les notes au démarrage
fn load_initial(handle: &AppHandle) -> Vec<Note> {
  let path = notes_file_path(handle);
  fs::read_to_string(&path)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

#[tauri::command]
fn list_notes(state: State<AppState>) -> Vec<Note> {
  state.notes.lock().unwrap().clone()
}

#[tauri::command]
fn add_note(content: String, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  let mut next_id = state.next_id.lock().unwrap();

  notes.push(Note { id: *next_id, content, pinned : false });
  *next_id += 1;

  let path = notes_file_path(&app_handle);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(&path, serde_json::to_string(&*notes).unwrap()).unwrap();

  window.emit("notes_updated", notes.clone()).unwrap();
}

#[tauri::command]
fn delete_note(id: u32, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  notes.retain(|n| n.id != id);

  let path = notes_file_path(&app_handle);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(&path, serde_json::to_string(&*notes).unwrap()).unwrap();

  window.emit("notes_updated", notes.clone()).unwrap();
}

#[tauri::command]
fn toggle_pin(id: u32, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
    note.pinned = !note.pinned;
  }

  let path = notes_file_path(&app_handle);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(&path, serde_json::to_string(&*notes).unwrap()).unwrap();

  window.emit("notes_updated", notes.clone()).unwrap();
}

fn main() {
  tauri::Builder::default()
    .plugin(UpdaterBuilder::new().build())
    .plugin(dialog_init())
    .plugin(process_init())
    .setup(|app| {
      let handle = app.handle();
      let initial = load_initial(&handle);
      let next_id = initial.iter().map(|n| n.id).max().unwrap_or(0) + 1;

      app.manage(AppState {
        notes: Mutex::new(initial),
        next_id: Mutex::new(next_id),
      });
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![list_notes, add_note, delete_note, toggle_pin])
    .run(tauri::generate_context!())
    .expect("Erreur au démarrage de l'application");
}