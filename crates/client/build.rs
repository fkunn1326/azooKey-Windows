fn main() {
    println!("cargo:rerun-if-changed=res/res.rc");
    println!("cargo:rerun-if-changed=res/res.h");
    let _ = embed_resource::compile("res/res.rc", embed_resource::NONE);
}
