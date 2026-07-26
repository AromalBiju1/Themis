#!/bin/bash
sed -i 's/match args.single::<UserId>() {/match crate::utils::parse_target(\&args.single::<String>().unwrap_or_default()).map(UserId::new) {/g' src/handlers/commands/modcmds.rs
sed -i 's/match args.single::<UserId>() {/match crate::utils::parse_target(\&args.single::<String>().unwrap_or_default()).map(UserId::new) {/g' src/handlers/commands/warns.rs

sed -i 's/Ok(id) => /Some(id) => /g' src/handlers/commands/modcmds.rs
sed -i 's/Ok(id) => /Some(id) => /g' src/handlers/commands/warns.rs

sed -i 's/Err(_) =>/None =>/g' src/handlers/commands/modcmds.rs
sed -i 's/Err(_) =>/None =>/g' src/handlers/commands/warns.rs

# specific fix for unban
sed -i 's/let user_id: UserId = match args.single() {/let user_id = match crate::utils::parse_target(\&args.single::<String>().unwrap_or_default()).map(UserId::new) {/g' src/handlers/commands/modcmds.rs
sed -i 's/Ok(id) => UserId::new(id),/Some(id) => id,/g' src/handlers/commands/modcmds.rs

# specific fix for purge filter
sed -i 's/let filter_uid = args.single::<UserId>().ok();/let filter_uid = crate::utils::parse_target(\&args.single::<String>().unwrap_or_default()).map(UserId::new);/g' src/handlers/commands/modcmds.rs
