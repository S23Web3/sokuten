//! Build script for Sokuten (速貼).
//!
//! Embeds the application icon and manifest via embed-resource.
//! Inputs: assets/sokuten.rc (resource script)
//! Outputs: linked icon + manifest in the final binary
//! Failure: embed_resource panics with a descriptive message if the .rc file is malformed

fn main() {
    // Only embed resources on Windows targets
    #[cfg(target_os = "windows")]
    {
        let _ = embed_resource::compile("assets/sokuten.rc", embed_resource::NONE);
    }
}
