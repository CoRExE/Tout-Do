// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Serialize, Deserialize};
use std::{fs, sync::Mutex};
use std::fs::File;
use std::io::{BufWriter, Write};
use tauri::{State, Window, Manager, Emitter, AppHandle};
use tauri::{
  menu::{Menu, MenuItem},
  tray::{TrayIconBuilder, TrayIconEvent}
};


// Plugins
use tauri_plugin_updater::Builder as UpdaterBuilder;
use tauri_plugin_dialog::init as dialog_init;
use tauri_plugin_process::init as process_init;
use tauri_plugin_notification::{init as notification_init, NotificationExt};

#[derive(Serialize, Deserialize, Clone)]
struct Note { id: u32, content: String, pinned: bool }

#[derive(Default)]
struct AppState {
  notes: Mutex<Vec<Note>>,
  next_id: Mutex<u32>,
  notif_enabled: Mutex<bool>,
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

// 💾 Sauvegarder les notes avec BufWriter pour de meilleures performances
fn save_notes(notes: &[Note], app_handle: &AppHandle) -> std::io::Result<()> {
  let path = notes_file_path(app_handle);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  let file = File::create(&path)?;
  let mut writer = BufWriter::new(file);
  serde_json::to_writer(&mut writer, notes)?;
  writer.flush()?;
  Ok(())
}

#[tauri::command]
fn list_notes(state: State<AppState>) -> Vec<Note> {
  let mut notes = state.notes.lock().unwrap().clone();
  // Trier les notes pour que les épinglées soient en premier
  notes.sort_by(|a, b| b.pinned.cmp(&a.pinned));
  notes
}

#[tauri::command]
fn add_note(content: String, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  let mut next_id = state.next_id.lock().unwrap();

  notes.push(Note { id: *next_id, content, pinned: false });
  *next_id += 1;

  if let Err(e) = save_notes(&notes, &app_handle) {
    eprintln!("Erreur lors de la sauvegarde des notes: {}", e);
  }

  window.emit("notes_updated", notes.clone()).unwrap();
}

#[tauri::command]
fn delete_note(id: u32, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  notes.retain(|n| n.id != id);

  if let Err(e) = save_notes(&notes, &app_handle) {
    eprintln!("Erreur lors de la sauvegarde des notes: {}", e);
  }

  window.emit("notes_updated", notes.clone()).unwrap();
}

#[tauri::command]
fn toggle_pin(id: u32, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
    note.pinned = !note.pinned;
  }

  if let Err(e) = save_notes(&notes, &app_handle) {
    eprintln!("Erreur lors de la sauvegarde des notes: {}", e);
  }

  window.emit("notes_updated", notes.clone()).unwrap();
}

#[tauri::command]
fn reorder_notes(ordered_ids: Vec<u32>, state: State<AppState>, app_handle: AppHandle, window: Window) {
  let mut notes = state.notes.lock().unwrap();
  
  // Créer un nouveau vecteur dans l'ordre spécifié
  let mut new_order = Vec::with_capacity(ordered_ids.len());
  let mut remaining_notes = std::mem::take(&mut *notes);
  
  for id in ordered_ids {
    if let Some(pos) = remaining_notes.iter().position(|n| n.id == id) {
      new_order.push(remaining_notes.remove(pos));
    }
  }
  
  // Ajouter les notes restantes (au cas où il y en aurait qui ne sont pas dans ordered_ids)
  new_order.extend(remaining_notes);
  
  *notes = new_order;

  if let Err(e) = save_notes(&*notes, &app_handle) {
    eprintln!("Erreur lors de la sauvegarde des notes: {}", e);
  }

  window.emit("notes_updated", notes.clone()).unwrap();
}

fn main() {
  tauri::Builder::default()
    .plugin(UpdaterBuilder::new().build())
    .plugin(dialog_init())
    .plugin(process_init())
    .plugin(notification_init())
    .setup(|app| {
      let handle = app.handle();
      let initial = load_initial(&handle);
      let next_id = initial.iter().map(|n| n.id).max().unwrap_or(0) + 1;

      app.manage(AppState {
        notes: Mutex::new(initial),
        next_id: Mutex::new(next_id),
        notif_enabled: Mutex::new(true),
      });
      let quit_item = MenuItem::with_id(app,"quit", "Quit", true, None::<&str>)?;
      let notif_item = MenuItem::with_id(app, "notif", "Notifications ✅", true, None::<&str>)?;
      let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
      let notif_item_clone = notif_item.clone();
      let menu = Menu::with_items(app, &[&show_item,&notif_item, &quit_item])?;
      let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|app, event| match event {
          TrayIconEvent::DoubleClick { .. } => {
            app.app_handle().show().unwrap();
          }
          _ => {}
        })
        .on_menu_event(move |app, event| {
          if event.id.as_ref() == "quit" {
            app.exit(0);
          } else if event.id.as_ref() == "notif" {
            let state = app.state::<AppState>();
            let mut notif_enabled = state.notif_enabled.lock().unwrap();
            *notif_enabled = !*notif_enabled;
            if *notif_enabled {
              notif_item_clone.set_text("Notifications ✅").unwrap();
              app.notification().builder().title("Tout-Do").body("Notifications activées").show().unwrap();
            } else {
              notif_item_clone.set_text("Notifications ❌").unwrap();
            }
          } else if event.id.as_ref() == "show" {
            app.show().unwrap();
          }
        })
        .build(app)?;
      if !app.notification().permission_state().is_ok() {
        app.notification().request_permission().unwrap();
      }
      app.notification()
        .builder()
        .title("Tout-Do")
        .body("L'application s'est lancé")
        .show()
        .unwrap();
      Ok(())
    })
    .on_window_event(|window, event| match event {
      tauri::WindowEvent::CloseRequested { api, .. } => {
        #[cfg(not(target_os = "macos"))]{
          window.hide().unwrap();
        }
        #[cfg(target_os = "macos")]{
          tauri::AppHandle::hide(&window.app_handle()).unwrap();
        }
        api.prevent_close();
      }
      _ => {}
    })
    .invoke_handler(tauri::generate_handler![
      list_notes,
      add_note,
      delete_note,
      toggle_pin,
      reorder_notes
    ])
    .run(tauri::generate_context!())
    .expect("Erreur au démarrage de l'application");
}