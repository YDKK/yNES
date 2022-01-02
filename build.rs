extern crate embed_resource;

#[cfg(target_os = "windows")]
fn main() {
    embed_resource::compile("src/win/resource.rc");
}