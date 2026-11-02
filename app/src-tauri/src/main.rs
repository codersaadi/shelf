#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod thumbs;

use state::AppState;
use tauri::Manager;

fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let state = match AppState::new(app.handle().clone()) {
                Ok(state) => state,
                Err(e) => {
                    eprintln!("shelf: cannot open the index: {e:#}");
                    std::process::exit(1);
                }
            };
            app.manage(state);
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("thumb", thumbs::handle)
        .invoke_handler(tauri::generate_handler![
            commands::list_shelf,
            commands::search,
            commands::parse_query,
            commands::get_files,
            commands::stats,
            commands::counts,
            commands::home,
            commands::inspect,
            commands::add_index_root,
            commands::set_ui_zoom,
            commands::close_window,
            commands::path_is_dir,
            commands::child_folders,
            commands::known_locations,
            commands::folder_info,
            commands::open_theme_file,
            commands::read_theme,
            commands::theme_file,
            commands::restore_trash,
            commands::rename_path,
            commands::purge_trash,
            commands::trash_entries,
            commands::open_data_folder,
            commands::transfer_paths,
            commands::open_terminal,
            commands::set_default_app,
            commands::set_default_mime,
            commands::default_apps,
            commands::settings_info,
            commands::text_snippet,
            commands::native_icon,
            commands::trash,
            commands::undo_trash,
            commands::cleanup_findings,
            commands::duplicates,
            commands::show_in_files,
            commands::open_file,
            commands::clear_activity,
            commands::open_with_apps,
            commands::open_with,
            commands::tags_of,
            commands::set_tags,
            commands::saved_searches,
            commands::save_search,
            commands::delete_saved_search,
            commands::index_root,
            commands::file_exists,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("shelf: failed to start: {e}");
        std::process::exit(1);
    }
}
