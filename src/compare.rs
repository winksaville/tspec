use std::path::Path;

use anyhow::Result;

use crate::binary::{binary_size, strip_binary};
use crate::cargo_build::{build_package, plain_cargo_build_release};
use crate::{print_header, print_hline};

/// Result of building a spec
pub struct SpecResult {
    pub name: String,
    pub size: u64,
}

/// Compare multiple specs for a package
pub fn compare_specs(
    pkg_name: &str,
    spec_paths: &[impl AsRef<Path> + std::fmt::Debug],
) -> Result<Vec<SpecResult>> {
    println!("Comparing {} builds:\n", pkg_name);

    let mut results = Vec::new();

    // Always build cargo --release baseline first (unstripped + stripped)
    match build_baseline(pkg_name) {
        Ok((size, stripped_size)) => {
            results.push(SpecResult {
                name: "cargo --release".to_string(),
                size,
            });
            results.push(SpecResult {
                name: "cargo --release-strip".to_string(),
                size: stripped_size,
            });
        }
        Err(_) => {
            println!("    baseline build failed, skipping");
        }
    }
    println!();

    for spec_path in spec_paths {
        let spec_path = spec_path.as_ref();
        let name = spec_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| spec_path.display().to_string());

        let size = build_spec(pkg_name, spec_path)?;
        results.push(SpecResult { name, size });
        println!();
    }

    // Sort by size (smallest first)
    results.sort_by_key(|r| r.size);

    Ok(results)
}

/// Build baseline and return (unstripped_size, stripped_size)
fn build_baseline(pkg_name: &str) -> Result<(u64, u64)> {
    println!("  cargo --release:");

    let build_result = plain_cargo_build_release(pkg_name)?;

    let size = binary_size(&build_result.binary_path)?;
    println!("    size: {} bytes", format_size(size));

    strip_binary(&build_result.binary_path)?;
    let stripped_size = binary_size(&build_result.binary_path)?;

    Ok((size, stripped_size))
}

fn build_spec(pkg_name: &str, spec_path: &Path) -> Result<u64> {
    let spec_str = spec_path.to_string_lossy();
    println!(
        "  {}:",
        spec_path.file_name().unwrap_or_default().to_string_lossy()
    );

    // Build using spec settings (profile, strip, etc. are all in the spec)
    let build_result = build_package(pkg_name, Some(&spec_str), None)?;

    let size = binary_size(&build_result.binary_path)?;
    println!("    size: {} bytes", format_size(size));

    Ok(size)
}

pub fn print_comparison(pkg_name: &str, results: &[SpecResult]) {
    let largest_size = results.iter().map(|r| r.size).max().unwrap_or(0);
    let max_name_len = results.iter().map(|r| r.name.len()).max().unwrap_or(4);

    // Format percent change: show reduction with minus sign, baseline as 0.0%
    let fmt_pct = |size: u64| -> String {
        if largest_size == 0 {
            return "   0.0%".to_string();
        }
        let pct = ((largest_size as f64 - size as f64) / largest_size as f64) * 100.0;
        if pct > 0.0 {
            format!("{:>7.1}%", -pct)
        } else {
            "   0.0%".to_string()
        }
    };

    println!();
    print_header!(format!("{} COMPARE SUMMARY", pkg_name));
    println!(
        "  {:width$}  {:>10}  {:>8}",
        "Spec",
        "Size",
        "Change",
        width = max_name_len
    );
    for result in results {
        println!(
            "  {:width$}  {:>10}  {}",
            result.name,
            format_size(result.size),
            fmt_pct(result.size),
            width = max_name_len
        );
    }
    print_hline!();
    println!();
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.3}M", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.3}K", bytes as f64 / 1_000.0)
    } else {
        format!("{}", bytes)
    }
}
