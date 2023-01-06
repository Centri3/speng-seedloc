mod seed;

use {
    rand::prelude::*,
    seed::seed,
    seedloc_handler::HANDLER,
    std::{
        error::Error,
        fs::{self, File},
        io::Write,
        thread,
        time::{Duration, Instant},
    },
    winput::{press, release, Button, Mouse},
};

// Address to the code of the selected object.
// Example: RS 0-3-397-1581-20880-7-556321-30 A3. Go visit it yourself! (:
const SELECTED_OBJECT_CODE: usize = 0x19a9e40usize;
// Pointer to the parameters of the selected object.
const SELECTED_OBJECT_POINTER: usize = 0x19a9ec0usize;
// Pointer to stars the Star browser found. seedloc will read this
const STAR_BROWSER_STARS_POINTER: usize = 0x1024440usize;
// Number of stars the Star browser has found
const STAR_BROWSER_STAR_LIST_LEN: usize = 0x102410cusize;
// Max stars the Star browser will search
const STAR_BROWSER_STAR_LIST_MAX: usize = 0x1024430usize;
// Whether the Star browser's currently searching; 0 = searching, 1 = idle
const STAR_BROWSER_SEARCHING: usize = 0x104a181usize;
// SE's current GUI scale.
const GUI_SCALE: usize = 0xe69434;

// Offsets from SELECTED_OBJECT_POINTER
const GALAXY_TYPE: usize = 0x8usize;
const GALAXY_SIZE: usize = 0x20usize;

// Coordinates to some GUI elements.
// These are always the same whenever ran, despite being initialized at runtime.
const STAR_BROWSER_SEARCH_BUTTON: usize = 0x1025a78usize;
const STAR_BROWSER_CLEAR_BUTTON: usize = 0x1025d60usize;

// Coordinates offsets
const GENERIC_OFFSET: i32 = 0xai32;
const WINDOWED_OFFSET: i32 = 0x14i32;

fn main() -> Result<(), Box<dyn Error>> {
    let mut finds = File::create("finds.log")?;

    let seeds = fs::read_to_string("seeds.txt")?
        .split(' ')
        .map(|s| s.parse::<i32>().unwrap())
        .collect::<Vec<i32>>();

    let mut rng = thread_rng();
    // This is easier to write 1000 times.
    let base = HANDLER.base();

    loop {
        // Select RG 0-3-397-1581, this is so we can reset the code of the currently
        // selected object. If we don't do this, it'll select nothing.
        HANDLER.run_script("select_rg_397.se", "Select \"RG 0-3-397-1581\"");

        // Not entirely sure how long we need to sleep for, but we need to give SE time
        // to update the currently selected object (Or anything else).
        thread::sleep(Duration::from_millis(160u64));

        loop {
            // Generate a random galaxy
            let level = rng.gen_range(1u32..9u32);
            let block = rng.gen_range(0u32..8u32.pow(level));
            let number = rng.gen_range(0u32..2500u32);

            // Write galaxy code to memory
            HANDLER.write(level, base + SELECTED_OBJECT_CODE + 0x4);
            HANDLER.write(block, base + SELECTED_OBJECT_CODE + 0x8);
            HANDLER.write(number, base + SELECTED_OBJECT_CODE + 0x10);

            thread::sleep(Duration::from_millis(160u64));

            let selected_object = HANDLER.read::<usize>(base + SELECTED_OBJECT_POINTER);

            // This could mean that the galaxy doesn't exist, or my code is too fast. Skip.
            // Also, skip any galaxies with a type of E/Irr or isn't 10% of max size
            if selected_object == 0usize
                // Ellipticals check
                || (1u32..=8u32).contains(&HANDLER.read(selected_object + GALAXY_TYPE))
                // Irregular check
                || HANDLER.read::<u32>(selected_object + GALAXY_TYPE) == 16u32
                // Size check
                || HANDLER.read::<f32>(selected_object + GALAXY_SIZE) <= 5000.0f32
            {
                continue;
            }

            let lat = rng.gen_range(-180.0f32..180.0f32);
            let dist_rad = rng.gen_range(0.2f32..0.5f32);

            // Goto the selected galaxy. If we've gotten this far, it's a desired galaxy
            HANDLER.run_script(
                "goto_galaxy_closer.se",
                format!("Goto {{ Lat {lat} Lon 90 Time 0 }}"),
            );

            thread::sleep(Duration::from_millis(160u64));

            // DistRad and Lat/Lon don't work together, for some reason
            HANDLER.run_script(
                "goto_galaxy_closer.se",
                format!("Goto {{ DistRad {dist_rad} Time 0 }}"),
            );

            thread::sleep(Duration::from_millis(160u64));

            // This is still vile.
            let scale = HANDLER.read::<f32>(base + GUI_SCALE);
            let search = (
                (HANDLER.read::<f32>(base + STAR_BROWSER_SEARCH_BUTTON) * scale) as i32
                    + GENERIC_OFFSET,
                (HANDLER.read::<f32>(base + STAR_BROWSER_SEARCH_BUTTON + 0x4) * scale) as i32
                    + GENERIC_OFFSET
                    + WINDOWED_OFFSET,
            );
            let clear = (
                (HANDLER.read::<f32>(base + STAR_BROWSER_CLEAR_BUTTON) * scale) as i32
                    + GENERIC_OFFSET,
                (HANDLER.read::<f32>(base + STAR_BROWSER_CLEAR_BUTTON + 0x4) * scale) as i32
                    + GENERIC_OFFSET
                    + WINDOWED_OFFSET,
            );

            Mouse::set_position(clear.0, clear.1)?;

            for _ in 0u32..=2u32 {
                press(Button::Left);

                thread::sleep(Duration::from_millis(32u64));

                release(Button::Left);
            }

            // Not entirely sure why, but I must wait a second or so for SE to catch up.
            thread::sleep(Duration::from_secs(1u64));

            Mouse::set_position(search.0, search.1)?;

            press(Button::Left);

            thread::sleep(Duration::from_millis(160u64));

            release(Button::Left);

            let star_list_max = HANDLER.read::<u32>(base + STAR_BROWSER_STAR_LIST_MAX);
            let star_list = HANDLER.read::<usize>(base + STAR_BROWSER_STARS_POINTER);

            let now = Instant::now();
            // Wait until systems found == max systems found, or until 10s has passed
            while HANDLER.read::<u32>(base + STAR_BROWSER_STAR_LIST_LEN) < star_list_max
                && now.elapsed().as_secs_f32() < 10.0f32
            {}

            // Stop search *hopefully* before it begins.
            HANDLER.write(1u8, base + STAR_BROWSER_SEARCHING);

            thread::sleep(Duration::from_millis(160u64));

            for i in 0usize..HANDLER.read::<u32>(base + STAR_BROWSER_STAR_LIST_LEN) as _ {
                let star = star_list + i * 0x78;

                // Get the code of the system
                let galaxy_universe_sector = HANDLER.read::<i32>(star + 0x10);
                let galaxy_level = HANDLER.read::<i32>(star + 0x14);
                let galaxy_block = HANDLER.read::<i32>(star + 0x18);
                let galaxy_number = HANDLER.read::<i32>(star + 0x20);
                let cluster_number = HANDLER.read::<i32>(star + 0x24);
                let galaxy_sector = HANDLER.read::<i32>(star + 0x28);
                let star_level = HANDLER.read::<i32>(star + 0x2C);
                let star_block = HANDLER.read::<i32>(star + 0x30);
                let star_number = HANDLER.read::<i32>(star + 0x38);
                let unflipped_x = HANDLER.read::<[u8; 16usize]>(star + 0x40);
                let unflipped_y = HANDLER.read::<[u8; 16usize]>(star + 0x50);
                let unflipped_z = HANDLER.read::<[u8; 16usize]>(star + 0x60);

                // We must flip the lower and upper 8-bytes of the star's coordinates
                let x = u128::from_le_bytes(
                    [&unflipped_x[8usize..], &unflipped_x[..8usize]]
                        .concat()
                        .try_into()
                        .unwrap(),
                );
                let y = u128::from_le_bytes(
                    [&unflipped_y[8usize..], &unflipped_y[..8usize]]
                        .concat()
                        .try_into()
                        .unwrap(),
                );
                let z = u128::from_le_bytes(
                    [&unflipped_z[8usize..], &unflipped_z[..8usize]]
                        .concat()
                        .try_into()
                        .unwrap(),
                );

                // Construct the code to the system. This is vile, but it works!
                let code = format!(
                    "{} {}-{}-{star_level}-{star_block}-{star_number}",
                    if cluster_number == -1i32 { "RS" } else { "RSC" },
                    if galaxy_universe_sector == -1i32 {
                        galaxy_number.to_string()
                    } else {
                        format_args!("{galaxy_universe_sector}-{galaxy_level}-{galaxy_block}-{galaxy_number}").to_string()
                    },
                    if cluster_number == -1i32 {
                        galaxy_sector.to_string()
                    } else {
                        cluster_number.to_string()
                    }
                );

                // Calculate the seed of the system, derived from the star's coordinates
                let seed = seed((x, y, z));

                if seeds.contains(&seed) {
                    finds.write_all(format!("CODE: {code}, SEED: {seed}\n").as_bytes())?;
                }
            }
        }
    }
}
