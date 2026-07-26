use serenity::all::UserId;
use serenity::framework::standard::Args;
fn main() {
    let mut args = Args::new("<@1516355651364716554>", &["<@1516355651364716554>"]);
    println!("{:?}", args.single::<UserId>());
    let mut args = Args::new("<@!1516355651364716554>", &["<@!1516355651364716554>"]);
    println!("{:?}", args.single::<UserId>());
}
