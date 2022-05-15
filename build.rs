fn main() {
    #[cfg(target_arch = "aarch64")]
    println!("cargo:rustc-link-arg=-lSystem");
}
