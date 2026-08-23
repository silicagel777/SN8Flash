fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        embed_resource::compile("src/res/app.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
