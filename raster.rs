//! Relleno de triángulos: coordenadas baricéntricas y líneas horizontales.
//!
//! Regla de cobertura común a ambos algoritmos (convención "top-left"):
//! un píxel (x, y) pertenece al triángulo si su centro (x + 0.5, y + 0.5)
//! está estrictamente dentro, o sobre una arista superior o izquierda.
//! Las aristas inferiores y derechas no se incluyen, así dos triángulos que
//! comparten una arista no generan fragmentos duplicados ni dejan huecos.
//! El eje Y crece hacia abajo (como en el framebuffer).

#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub x: i32,
    pub y: i32,
    pub color: u32,
}

#[derive(Clone, Copy)]
pub enum TriangleMode {
    Wireframe,
    FilledBarycentric,
    FilledScanline,
}

pub type Triangle = [Vec2; 3];

const EPS: f64 = 1e-9;

/// Rasteriza todos los triángulos con el modo indicado.
/// Solo se generan fragmentos dentro del framebuffer (width x height).
pub fn rasterize(
    triangles: &[Triangle],
    mode: TriangleMode,
    width: i32,
    height: i32,
    color: u32,
) -> Vec<Fragment> {
    let mut out = Vec::new();
    for t in triangles {
        match mode {
            TriangleMode::Wireframe => out.extend(triangle_wireframe(t[0], t[1], t[2], width, height, color)),
            TriangleMode::FilledBarycentric => {
                out.extend(triangle_barycentric(t[0], t[1], t[2], width, height, color))
            }
            TriangleMode::FilledScanline => {
                out.extend(triangle_scanline(t[0], t[1], t[2], width, height, color))
            }
        }
    }
    out
}

/// Línea de Bresenham entre dos puntos (redondeados a enteros).
pub fn line(a: Vec2, b: Vec2, color: u32) -> Vec<Fragment> {
    let (mut x0, mut y0) = (a.x.round() as i32, a.y.round() as i32);
    let (x1, y1) = (b.x.round() as i32, b.y.round() as i32);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut out = Vec::new();
    loop {
        out.push(Fragment { x: x0, y: y0, color });
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
    out
}

/// Contorno: tres llamadas a `line`.
pub fn triangle_wireframe(a: Vec2, b: Vec2, c: Vec2, width: i32, height: i32, color: u32) -> Vec<Fragment> {
    let mut out = Vec::new();
    out.extend(line(a, b, color));
    out.extend(line(b, c, color));
    out.extend(line(c, a, color));
    out.retain(|f| f.x >= 0 && f.y >= 0 && f.x < width && f.y < height);
    out
}

/// Función de arista: doble del área con signo del triángulo (a, b, p).
#[inline]
fn edge(a: Vec2, b: Vec2, p: Vec2) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// Una arista a->b (con área total positiva) es "top-left" si es horizontal
/// con el interior debajo, o si sube en pantalla (interior a su derecha).
#[inline]
fn is_top_left(a: Vec2, b: Vec2) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dy == 0.0 && dx > 0.0) || dy < 0.0
}

/// Algoritmo 1: coordenadas baricéntricas sobre el rectángulo envolvente.
#[inline(never)]
pub fn triangle_barycentric(a: Vec2, b: Vec2, mut c: Vec2, width: i32, height: i32, color: u32) -> Vec<Fragment> {
    let mut b = b;
    let mut area = edge(a, b, c);
    if area == 0.0 {
        return Vec::new(); // caso degenerado: sin interior
    }
    if area < 0.0 {
        // Orden inverso: se intercambian dos vértices para tener área positiva.
        std::mem::swap(&mut b, &mut c);
        area = -area;
    }

    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
    let max_x = (a.x.max(b.x).max(c.x).ceil() as i32).min(width - 1);
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i32).min(height - 1);

    let tl0 = is_top_left(b, c);
    let tl1 = is_top_left(c, a);
    let tl2 = is_top_left(a, b);

    let mut out = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vec2::new(x as f64 + 0.5, y as f64 + 0.5);
            let w0 = edge(b, c, p); // peso del vértice a (sin dividir por el área)
            let w1 = edge(c, a, p); // peso del vértice b
            let w2 = edge(a, b, p); // peso del vértice c
            let inside = (w0 > 0.0 || (w0 == 0.0 && tl0))
                && (w1 > 0.0 || (w1 == 0.0 && tl1))
                && (w2 > 0.0 || (w2 == 0.0 && tl2));
            if inside {
                // Aquí las coordenadas baricéntricas serían (w0/area, w1/area, w2/area)
                // para interpolar atributos más adelante.
                let _ = area;
                out.push(Fragment { x, y, color });
            }
        }
    }
    out
}

/// Algoritmo 2: una fila a la vez, intersectando aristas con y = fila + 0.5.
///
/// Regla de aristas: una arista cruza la fila si lo.y <= yc < hi.y (las
/// horizontales nunca cruzan; un vértice exactamente en la fila cuenta solo
/// para la arista que parte de él hacia abajo). En x, el tramo es [xa, xb):
/// incluye el extremo izquierdo y excluye el derecho.
#[inline(never)]
pub fn triangle_scanline(a: Vec2, b: Vec2, c: Vec2, width: i32, height: i32, color: u32) -> Vec<Fragment> {
    if edge(a, b, c) == 0.0 {
        return Vec::new(); // degenerado
    }
    let edges = [(a, b), (b, c), (c, a)];
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i32).min(height - 1);

    let mut out = Vec::new();
    for y in min_y..=max_y {
        let yc = y as f64 + 0.5;
        let mut xs = [0.0f64; 2];
        let mut n = 0;
        for &(p, q) in &edges {
            let (lo, hi) = if p.y < q.y { (p, q) } else { (q, p) };
            if lo.y != hi.y && lo.y <= yc && yc < hi.y && n < 2 {
                let t = (yc - lo.y) / (hi.y - lo.y);
                xs[n] = lo.x + t * (hi.x - lo.x);
                n += 1;
            }
        }
        if n < 2 {
            continue;
        }
        let (xa, xb) = if xs[0] <= xs[1] { (xs[0], xs[1]) } else { (xs[1], xs[0]) };
        // Píxeles con xa <= x + 0.5 < xb
        let start = ((xa - 0.5 - EPS).ceil() as i32).max(0);
        let end = ((xb - 0.5 - EPS).ceil() as i32 - 1).min(width - 1);
        for x in start..=end {
            out.push(Fragment { x, y, color });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 200;
    const H: i32 = 150;
    const C: u32 = 0xFFFFFF;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn keys(f: &[Fragment]) -> Vec<(i32, i32)> {
        let mut k: Vec<_> = f.iter().map(|f| (f.x, f.y)).collect();
        k.sort();
        k
    }

    fn assert_same(a: Vec2, b: Vec2, c: Vec2) -> usize {
        let f1 = triangle_barycentric(a, b, c, W, H, C);
        let f2 = triangle_scanline(a, b, c, W, H, C);
        let (k1, k2) = (keys(&f1), keys(&f2));
        assert_eq!(k1, k2, "difieren para {a:?} {b:?} {c:?}");
        let mut d = k2.clone();
        d.dedup();
        assert_eq!(d.len(), k2.len(), "fragmentos duplicados");
        k1.len()
    }

    #[test]
    fn pequeno_y_grande() {
        assert!(assert_same(v(10.0, 10.0), v(14.0, 10.0), v(10.0, 14.0)) > 0);
        assert!(assert_same(v(5.0, 5.0), v(190.0, 20.0), v(80.0, 140.0)) > 1000);
    }

    #[test]
    fn borde_horizontal_y_vertical() {
        assert!(assert_same(v(20.0, 30.0), v(100.0, 30.0), v(60.0, 90.0)) > 0);
        assert!(assert_same(v(20.0, 30.0), v(20.0, 100.0), v(90.0, 60.0)) > 0);
    }

    #[test]
    fn orden_inverso() {
        let (a, b, c) = (v(12.3, 8.7), v(120.1, 40.4), v(50.9, 110.2));
        let n1 = assert_same(a, b, c);
        let n2 = assert_same(a, c, b);
        assert_eq!(n1, n2);
        assert!(n1 > 0);
    }

    #[test]
    fn parcialmente_fuera() {
        let n = assert_same(v(-40.0, -30.0), v(150.0, 60.0), v(60.0, 200.0));
        assert!(n > 0);
        let f = triangle_scanline(v(-40.0, -30.0), v(150.0, 60.0), v(60.0, 200.0), W, H, C);
        assert!(f.iter().all(|f| f.x >= 0 && f.y >= 0 && f.x < W && f.y < H));
    }

    #[test]
    fn degenerado() {
        assert_eq!(assert_same(v(10.0, 10.0), v(50.0, 50.0), v(90.0, 90.0)), 0);
        assert_eq!(assert_same(v(10.0, 10.0), v(10.0, 10.0), v(10.0, 10.0)), 0);
    }

    #[test]
    fn aristas_compartidas_sin_huecos_ni_duplicados() {
        // Cuadrado dividido en dos triángulos por la diagonal.
        let (p0, p1, p2, p3) = (v(10.0, 10.0), v(60.0, 10.0), v(60.0, 60.0), v(10.0, 60.0));
        for mode in [TriangleMode::FilledBarycentric, TriangleMode::FilledScanline] {
            let f = rasterize(&[[p0, p1, p2], [p0, p2, p3]], mode, W, H, C);
            let k = keys(&f);
            let mut d = k.clone();
            d.dedup();
            assert_eq!(d.len(), k.len());
            assert_eq!(k.len(), 50 * 50);
        }
    }

    #[test]
    fn aleatorios_enteros_y_reales() {
        let mut s: u64 = 12345;
        let mut r = |m: f64| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f64 / (1u64 << 31) as f64) * m
        };
        for i in 0..20000 {
            let (mut p, mut q, mut w) = ([0.0; 2], [0.0; 2], [0.0; 2]);
            for k in 0..2 {
                let m = if k == 0 { 260.0 } else { 200.0 };
                p[k] = r(m) - 30.0;
                q[k] = r(m) - 30.0;
                w[k] = r(m) - 30.0;
            }
            if i % 2 == 0 {
                for a in [&mut p, &mut q, &mut w] {
                    a[0] = a[0].round();
                    a[1] = a[1].round();
                }
            }
            assert_same(v(p[0], p[1]), v(q[0], q[1]), v(w[0], w[1]));
        }
    }
}
