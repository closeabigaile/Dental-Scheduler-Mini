// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(debug_assertions)]
  {
  let args: Vec<_> = std::env::args_os().collect();
  if args.get(1).is_some_and(|v| v == "--migration-verify") {
    let Some(path) = args.get(2).filter(|_| args.len() == 3) else {
      eprintln!("Use --migration-verify <SQLite test/copy path>");
      std::process::exit(2);
    };
    std::process::exit(dental_scheduler_mini_lib::migration_verification::run_cli(std::path::Path::new(path)));
  }
  }
  dental_scheduler_mini_lib::run();
}
