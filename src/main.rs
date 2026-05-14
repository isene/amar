//! amar — Amar RPG companion (Fe2O3 suite).

#![allow(dead_code)]  // PC sheet edit / Forge generators / Inspire prompts populate dead code in v0.2+.

mod adventure;
mod app;
mod calendar;
mod canon;
mod dice;
mod forge;
mod lore;
mod pc;
mod portrait;
mod store;
mod theme;

fn main() {
    // CLI bootstrap mode — non-TUI. Lets the user populate a
    // campaign from the shell without walking the menu tree.
    // Recognised flags (all optional, but at least one must be
    // present to enter this branch):
    //   --import <camp-name> <adventure-dir>
    //       Make sure the campaign exists, scan the directory into
    //       a new Adventure, append + save. Also marks the imported
    //       adventure as ACTIVE if the campaign had none.
    //   --current-section <heading-substring>
    //       After import, set the imported adventure's
    //       `current_section` to the first section whose heading
    //       contains the substring (case-insensitive).
    //   --create-campaign <name>
    //       Create an empty campaign with the given name (idempotent).
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a|
        a == "--import" || a == "--create-campaign" || a == "--rescan-all")
    {
        match cli_bootstrap(&args) {
            Ok(msg) => { println!("{}", msg); return; }
            Err(e)  => { eprintln!("Bootstrap failed: {}", e); std::process::exit(1); }
        }
    }
    crust::Crust::set_app_identity("Amar");
    crust::Crust::init();
    let mut app = app::App::new();
    app.run();
    crust::Crust::cleanup();
}

/// Argument-driven campaign / adventure population. Tiny — no
/// dependencies beyond what the TUI binary already pulls in. Returns
/// a human-readable summary on success.
fn cli_bootstrap(args: &[String]) -> Result<String, String> {
    let mut camp_name: Option<String> = None;
    let mut adventure_dir: Option<std::path::PathBuf> = None;
    let mut current_section: Option<String> = None;
    let mut create_only: Option<String> = None;
    let mut rescan_all: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--import" if i + 2 < args.len() => {
                camp_name = Some(args[i + 1].clone());
                adventure_dir = Some(std::path::PathBuf::from(&args[i + 2]));
                i += 3;
            }
            "--current-section" if i + 1 < args.len() => {
                current_section = Some(args[i + 1].clone());
                i += 2;
            }
            "--create-campaign" if i + 1 < args.len() => {
                create_only = Some(args[i + 1].clone());
                i += 2;
            }
            "--rescan-all" if i + 1 < args.len() => {
                rescan_all = Some(args[i + 1].clone());
                i += 2;
            }
            _ => i += 1,
        }
    }

    // Re-walk every adventure in the named campaign so the latest
    // attachment matcher / parser / NPC promotion runs in-place.
    // Preserves current_section + per-section + per-adventure notes
    // by heading match. Idempotent.
    if let Some(name) = rescan_all.as_ref() {
        let mut camp = store::Campaign::load(name)
            .map_err(|e| format!("load campaign '{}': {}", name, e))?;
        let mut total_refreshed = 0usize;
        let mut total_attached = 0usize;
        for idx in 0..camp.adventures.len() {
            let (root, id, adv_name) = {
                let a = &camp.adventures[idx];
                (a.root_dir.clone(), a.id, a.name.clone())
            };
            match adventure::import_from_dir(std::path::Path::new(&root), id) {
                Ok(mut new_adv) => {
                    {
                        let old = &camp.adventures[idx];
                        new_adv.current_section = old.current_section;
                        new_adv.notes = old.notes.clone();
                        for new_sec in new_adv.sections.iter_mut() {
                            if let Some(old_sec) = old.sections.iter()
                                .find(|s| s.heading == new_sec.heading)
                            {
                                new_sec.notes = old_sec.notes.clone();
                            }
                        }
                    }
                    let n_attached: usize = new_adv.sections.iter()
                        .map(|s| s.attached_images.len()).sum();
                    total_attached += n_attached;
                    camp.adventures[idx] = new_adv;
                    camp.promote_adventure_portraits_to_npcs(idx);
                    total_refreshed += 1;
                    eprintln!("  refreshed '{}' — {} attached images",
                        adv_name, n_attached);
                }
                Err(e) => eprintln!("  skipped '{}' ({})", adv_name, e),
            }
        }
        camp.save().map_err(|e| format!("save: {}", e))?;
        return Ok(format!(
            "Rescanned {}/{} adventures in '{}' ({} images attached).",
            total_refreshed, camp.adventures.len(), name, total_attached));
    }

    if let Some(name) = create_only.as_ref() {
        let mut camp = store::Campaign::load(name)
            .unwrap_or_else(|_| store::Campaign::new(name));
        camp.save().map_err(|e| format!("save failed: {}", e))?;
        // Flip the global active-campaign pointer here too so a
        // `--create-campaign` on its own is enough to point amar at
        // the new campaign on next launch.
        let mut cfg = store::GlobalConfig::load();
        cfg.active_campaign = Some(name.clone());
        cfg.save().map_err(|e| format!("config save failed: {}", e))?;
        if camp_name.is_none() && adventure_dir.is_none() {
            return Ok(format!("Campaign '{}' ready at {} (set active)",
                name,
                store::campaign_dir(name).display()));
        }
    }

    let name = camp_name.ok_or_else(||
        "--import requires <camp-name> <adventure-dir>".to_string())?;
    let dir = adventure_dir.ok_or_else(||
        "--import requires <camp-name> <adventure-dir>".to_string())?;

    let mut camp = store::Campaign::load(&name)
        .unwrap_or_else(|_| store::Campaign::new(&name));
    let next_id = camp.adventures.iter().map(|a| a.id).max().map(|n| n + 1).unwrap_or(1);
    let mut adv = adventure::import_from_dir(&dir, next_id)?;
    if let Some(substr) = current_section.as_ref() {
        let needle = substr.to_lowercase();
        let idx = adv.sections.iter().position(|s|
            s.heading.to_lowercase().contains(&needle));
        if let Some(i) = idx {
            adv.current_section = Some(i);
        } else {
            return Err(format!(
                "no section heading matches '{}' in {}",
                substr, adv.name));
        }
    }
    let n_sec = adv.sections.len();
    let n_assets = adv.scenes.len() + adv.floorplans.len()
        + adv.npc_portraits.len() + adv.npc_docs.len();
    let adv_name = adv.name.clone();
    let new_id = adv.id;
    let was_first = camp.active_adventure_id.is_none();
    camp.adventures.push(adv);
    let new_idx = camp.adventures.len() - 1;
    if was_first {
        camp.active_adventure_id = Some(new_id);
    }
    let n_new_npcs = camp.promote_adventure_portraits_to_npcs(new_idx);
    camp.save().map_err(|e| format!("save failed: {}", e))?;
    // Flip the global active-campaign pointer so the next interactive
    // launch loads what we just imported into, instead of whatever
    // campaign the previous session left in config.
    let mut cfg = store::GlobalConfig::load();
    cfg.active_campaign = Some(name.clone());
    cfg.save().map_err(|e| format!("config save failed: {}", e))?;
    Ok(format!(
        "Imported '{}' into campaign '{}' — {} sections, {} assets, {} new NPCs. \
         Active = {}. Set as the active campaign on next launch.",
        adv_name, name, n_sec, n_assets, n_new_npcs,
        if was_first { "yes" } else { "unchanged" }))
}
