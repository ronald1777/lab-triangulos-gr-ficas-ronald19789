mod raster;

use raster::{rasterize, Triangle, TriangleMode, Vec2};
use std::hint::black_box;
use std::io::Write;
use std::time::Instant;

const W: i32 = 800;
const H: i32 = 600;
const COLOR: u32 = 0x00E6_A23C;

/// Triángulos deterministas de distintos tamaños para medir.
fn bench_triangles() -> Vec<Triangle> {
    let mut s: u64 = 42;
    let mut r = |m: f64| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 33) as f64 / (1u64 << 31) as f64) * m
    };
    (0..200)
        .map(|i| {
            let size = if i % 4 == 0 { 300.0 } else { 40.0 };
            let (cx, cy) = (r(W as f64), r(H as f64));
            [
                Vec2::new(cx + r(size), cy + r(size)),
                Vec2::new(cx + r(size), cy + r(size)),
                Vec2::new(cx + r(size), cy + r(size)),
            ]
        })
        .collect()
}

fn save_ppm(path: &str, frags: &[raster::Fragment]) {
    let mut img = vec![[20u8, 20, 28]; (W * H) as usize];
    for f in frags {
        img[(f.y * W + f.x) as usize] = [(f.color >> 16) as u8, (f.color >> 8) as u8, f.color as u8];
    }
    let mut file = std::fs::File::create(path).unwrap();
    write!(file, "P6\n{W} {H}\n255\n").unwrap();
    for p in img {
        file.write_all(&p).unwrap();
    }
}

fn render_images() {
    std::fs::create_dir_all("images").unwrap();
    let tris: Vec<Triangle> = vec![
        [Vec2::new(100.0, 100.0), Vec2::new(380.0, 140.0), Vec2::new(200.0, 330.0)],
        [Vec2::new(450.0, 80.0), Vec2::new(720.0, 80.0), Vec2::new(590.0, 300.0)],
        [Vec2::new(120.0, 400.0), Vec2::new(120.0, 560.0), Vec2::new(300.0, 560.0)],
        [Vec2::new(520.0, 380.0), Vec2::new(860.0, 450.0), Vec2::new(600.0, 640.0)], // parcialmente fuera
    ];
    for (name, mode) in [
        ("wireframe", TriangleMode::Wireframe),
        ("barycentric", TriangleMode::FilledBarycentric),
        ("scanline", TriangleMode::FilledScanline),
    ] {
        let frags = rasterize(&tris, mode, W, H, COLOR);
        save_ppm(&format!("images/{name}.ppm"), &frags);
        println!("{name}: {} fragmentos", frags.len());
    }
}

fn median(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn measure() {
    let triangles = bench_triangles();
    println!("Triángulos: {}, framebuffer {W}x{H}", triangles.len());
    for (name, mode) in [
        ("Baricéntrico", TriangleMode::FilledBarycentric),
        ("Scanline", TriangleMode::FilledScanline),
    ] {
        let count = rasterize(&triangles, mode, W, H, COLOR).len();
        let mut times = Vec::new();
        for _ in 0..11 {
            let start = Instant::now();
            for _ in 0..100 {
                let fragments = rasterize(black_box(&triangles), mode, W, H, COLOR);
                black_box(fragments);
            }
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        println!(
            "{name}: fragmentos por ejecución = {count}, mediana de 100 ejecuciones = {:.2} ms (11 repeticiones)",
            median(&mut times)
        );
    }
}

/// Ejecución sin ventana para el perfilador.
fn benchmark_triangles() {
    let triangles = bench_triangles();
    let mut total = 0usize;
    for _ in 0..500 {
        for mode in [TriangleMode::FilledBarycentric, TriangleMode::FilledScanline] {
            total += black_box(rasterize(black_box(&triangles), mode, W, H, COLOR)).len();
        }
    }
    println!("fragmentos totales: {total}");
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--benchmark-triangles") => benchmark_triangles(),
        Some("--measure") => measure(),
        _ => render_images(),
    }
}
