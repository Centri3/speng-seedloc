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
        time::Duration,
    },
};

// Address to the code of the selected object.
// Example: RS 0-3-397-1581-20880-7-556321-30 A3. Go visit it yourself! (:
const SELECTED_OBJECT_CODE: usize = 0x19a9e40usize;
// Pointer to the parameters of the selected object.
const SELECTED_OBJECT_POINTER: usize = 0x19a9ec0usize;
// Pointer to the star node cache.
const STAR_NODE_CACHE_POINTER: usize = 0x7b9eb8;

// Offsets from SELECTED_OBJECT_POINTER
const GALAXY_TYPE: usize = 0x8usize;
const GALAXY_SIZE: usize = 0x20usize;
const GALAXY_INDEX: usize = 0x66usize;

fn main() -> Result<(), Box<dyn Error>> {
    let mut finds = File::create("finds.log")?;

    let seeds_txt = fs::read_to_string("seeds.txt")?;
    // What the fuck
    let seeds = seeds_txt
        .lines()
        .map(|l| {
            // This will split at whitespace, can't use .split_whitespace() because
            // .as_str() isn't stable! <https://github.com/rust-lang/rust/issues/77998>
            let line = l.split_once(' ').unwrap();

            // Isolate both the seed and types of stars
            let seed = line.0;
            let types = line.1;

            (
                seed.parse::<i32>().unwrap(),
                // Since we used .split_once() earlier, we can .split() again to get an iterator
                // over each star type. Spaghetti, but it works!
                types
                    .split(' ')
                    .map(|t| t.parse::<u16>().unwrap())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<(_, Vec<_>)>>();

    let mut rng = thread_rng();
    // This is easier to write 1000 times.
    let base = HANDLER.base();

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

        // Get the address to the selected object
        let selected_object = HANDLER.read::<usize>(base + SELECTED_OBJECT_POINTER);

        // This could mean that the galaxy doesn't exist, or my code is too fast. Skip.
        // Also, skip any galaxies with a type of E/Irr or isn't 25% of max size
        if selected_object == 0usize
            || (1u32..=8u32).contains(&HANDLER.read(selected_object + GALAXY_TYPE))
            || HANDLER.read::<u32>(selected_object + GALAXY_TYPE) == 16u32
            || HANDLER.read::<f32>(selected_object + GALAXY_SIZE) <= 12500.0f32
        {
            continue;
        }

        // Goto the same galaxy 4 times, this is so we can load as much of the galaxy at
        // once as possible. Only once is needed, but the later 3 help load more stars
        for i in 0i32..4i32 {
            HANDLER.run_script(
                "goto_galaxy.se",
                format!("Goto {{ Lat {} Lon 90 Time 0 }}", 90.0f32 * i as f32),
            );

            thread::sleep(Duration::from_millis(80u64));

            // DistRad and Lat/Lon don't work together, for some reason
            HANDLER.run_script("goto_galaxy_closer.se", "Goto { DistRad 0.4 Time 0 }");

            // We must wait to let SE load in stars, this sometimes silently
            // fails (it takes forever to load), so we will wait only a tiny bit
            thread::sleep(Duration::from_millis(320u64));
        }

        // Index of the galaxy into the star node cache, I think...
        let galaxy_index = HANDLER.read::<u8>(selected_object + GALAXY_INDEX);
        // Pointer to the star node cache
        let star_node_cache = HANDLER.read::<usize>(base + STAR_NODE_CACHE_POINTER);

        // Get the address to the galaxy's sectors
        let galaxy_sectors = HANDLER
            .read::<usize>(star_node_cache + galaxy_index as usize * 0x1b0usize + 0x130usize);

        // I'm roughly certain 30000 is a little over the max id for a sector
        for i in 0usize..30000usize {
            // Each sector's size is 900 bytes
            let sector = galaxy_sectors + (i * 0x60usize);

            // Address to the top level star node
            let star_node = HANDLER.read::<usize>(sector + 0x58usize);

            // If the sector doesn't exist, its star node will equal NULL
            if star_node == 0usize {
                continue;
            }

            // We can't overwrite the previous i here, as we need it when printing the
            // star's code. So c it is! (Best letter of the alphabet. Fight me)
            for c in 1usize..=8usize {
                let child = HANDLER.read::<usize>(star_node + 0x8usize * c);

                println!("RS 0-{level}-{block}-{number}-{i}-{c}: {child:x}");
            }

            panic!();
        }
    }

    /*

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
                        format_args!(
                        "{galaxy_universe_sector}-{galaxy_level}-{galaxy_block}-{galaxy_number}"
                    )
                        .to_string()
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
    */
}
