use std::fs;

pub fn repair() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = crate::paths::data_dir();
    let bin_dir = crate::paths::bin_dir();

    let base_new = data_dir.join("catalog").join("base.new");
    let base_old = data_dir.join("catalog").join("base.old");
    let bin_old = bin_dir.join("edash.old");
    let bin_path = bin_dir.join("edash");
    let catalog_base = crate::paths::catalog_base_dir();

    let mut repaired = false;

    // base.old + edash.old → previous update failed after swap; restore old binary, keep new catalog
    if base_old.exists() && bin_old.exists() {
        println!("Found interrupted update (base.old + edash.old) — restoring old binary...");
        if bin_path.exists() {
            let _ = fs::remove_file(&bin_path);
        }
        if let Err(e) = fs::rename(&bin_old, &bin_path) {
            eprintln!("  ✗ failed to restore binary: {e}");
        } else {
            println!("  ✓ restored previous binary");
        }
        // Keep new catalog (already validated)
        if base_old.exists() {
            let _ = fs::remove_dir_all(&base_old);
            println!("  ✓ cleaned up base.old");
        }
        repaired = true;
    }

    // base.new only → staging was interrupted, discard it
    if base_new.exists() {
        println!("Found incomplete catalog staging (base.new) — discarding...");
        if let Err(e) = fs::remove_dir_all(&base_new) {
            eprintln!("  ✗ failed to remove base.new: {e}");
        } else {
            println!("  ✓ removed base.new");
        }
        repaired = true;
    }

    // edash.old only → binary swap interrupted, restore old binary
    if bin_old.exists() && !base_old.exists() {
        println!("Found interrupted binary swap (edash.old) — restoring old binary...");
        if bin_path.exists() {
            let _ = fs::remove_file(&bin_path);
        }
        if let Err(e) = fs::rename(&bin_old, &bin_path) {
            eprintln!("  ✗ failed to restore binary: {e}");
        } else {
            println!("  ✓ restored previous binary");
        }
        repaired = true;
    }

    // base.old only without bin.old — catalog swap happened, binary didn't; catalog is live
    // This is actually fine — catalog is at base/, binary is old. Keep both.
    if base_old.exists() && !bin_old.exists() {
        println!("Found stray base.old (catalog swap completed, binary unchanged) — cleaning up...");
        if catalog_base.exists() {
            let _ = fs::remove_dir_all(&base_old);
            println!("  ✓ removed base.old (current catalog is live)");
        } else {
            // base/ doesn't exist — restore from base.old
            if let Err(e) = fs::rename(&base_old, &catalog_base) {
                eprintln!("  ✗ failed to restore catalog: {e}");
            } else {
                println!("  ✓ restored catalog from base.old");
            }
        }
        repaired = true;
    }

    if !repaired {
        println!("Nothing to repair.");
    }

    Ok(())
}

/// Check for stale files from interrupted updates and repair if needed.
/// Called automatically at startup. Returns true if a repair was performed.
pub fn auto_repair_if_needed() -> bool {
    let data_dir = crate::paths::data_dir();
    let bin_dir = crate::paths::bin_dir();

    let base_new = data_dir.join("catalog").join("base.new");
    let base_old = data_dir.join("catalog").join("base.old");
    let bin_old = bin_dir.join("edash.old");

    let needs_repair = base_new.exists() || base_old.exists() || bin_old.exists();

    if needs_repair {
        eprintln!("edash: found leftover files from an interrupted update. Run 'edash repair'.");
    }

    needs_repair
}
