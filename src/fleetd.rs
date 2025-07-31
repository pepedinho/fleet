use std::{fs, os::unix::net::UnixListener, path::Path};

use clap::Parser;

use crate::{app::handle_watch, cli::{Cli, Commands}, ipc::server::handle_request};

mod cli;
mod config;
mod app;
mod git;
mod exec;
mod core;
mod ipc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // println!("Fleet Daemon started.");

    // // Charger l’état depuis ~/.fleetd/state.json (à venir)
    // // Démarrer les watchers pour chaque projet enregistré

    // // TODO: Écouter sur un socket (IPC) pour recevoir des ordres du CLI

    // // TODO: Rafraîchir les projets régulièrement
    // loop {
    //     // stub: remplacer plus tard par boucle async avec tokio
    //     std::thread::sleep(std::time::Duration::from_secs(60));
    // }

    let socket_path = "/tmp/fleetd.sock";
    
    if Path::new(socket_path).exists() {
        fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(socket_path)?;
    println!("fleetd is listening ...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || {
                    handle_request(stream).unwrap_or_else(|e| eprintln!("❌ Error: {:?}", e));
                });
            }
            Err(e) => {
                eprintln!("❌ Connexion failed: {:?}", e);
            }
        }
    }
    
    Ok(())
}