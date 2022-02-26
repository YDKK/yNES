#[cfg(windows)]
extern crate embed_resource;

fn main() {
    #[cfg(windows)]
    embed_resource::compile("src/win/resource.rc");
}