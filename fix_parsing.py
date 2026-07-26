import os

files = ['src/handlers/commands/warns.rs', 'src/handlers/commands/modcmds.rs']
for f in files:
    with open(f, 'r') as file:
        content = file.read()
    
    # Replace the generic args.single::<UserId>() 
    content = content.replace("args.single::<UserId>()", "crate::utils::parse_target_from_args(&mut args).map(UserId::new)")
    
    # Unban uses implicit type inference
    content = content.replace("let user_id: UserId = match args.single() {", "let user_id: UserId = match crate::utils::parse_target_from_args(&mut args).map(UserId::new) {")

    with open(f, 'w') as file:
        file.write(content)
