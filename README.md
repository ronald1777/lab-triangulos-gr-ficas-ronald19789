# Laboratorio: rellenar triángulos

**Nombre:** Ronald Catún  
**Carnet:** 19789

Se implementaron dos algoritmos de relleno de triángulos en Rust (`src/raster.rs`), además del modo de contorno (`Wireframe`, tres llamadas a `line`). El modo se elige con `TriangleMode` y `rasterize` devuelve un `Vec<Fragment>`.

## Cómo ejecutar

```
cargo test --release                       # comparación de ambos algoritmos
cargo run --release                        # genera images/*.png (contorno y rellenos)
cargo run --release -- --measure           # mediciones de tiempo
cargo flamegraph -- --benchmark-triangles  # perfil (ejecución sin ventana)
```

## Regla de cobertura (igual en ambos algoritmos)

Un píxel `(x, y)` se evalúa en su centro `(x + 0.5, y + 0.5)`. Pertenece al triángulo si el centro está estrictamente dentro, o sobre una arista **superior o izquierda** (convención top-left, con Y hacia abajo). Las aristas inferiores y derechas no se incluyen. Así dos triángulos que comparten una arista no producen fragmentos duplicados ni huecos.

- **Baricéntrico:** recorre el rectángulo envolvente (recortado al framebuffer) y calcula las tres funciones de arista en cada centro. Si el área es 0 devuelve un `Vec` vacío (caso degenerado). Si el área es negativa (orden inverso) intercambia dos vértices, por lo que el triángulo no desaparece. Las igualdades exactas a 0 se resuelven con la regla top-left.
- **Líneas horizontales:** para cada fila evalúa `y = fila + 0.5`. Una arista cruza la fila si `lo.y <= y < hi.y`; las aristas horizontales no cruzan nunca y un vértice exactamente en la fila cuenta solo para la arista que sale de él hacia abajo. En x, el tramo es `[xa, xb)`: se incluye el extremo izquierdo y se excluye el derecho (con una tolerancia de 1e-9 para los empates).

## Imágenes

| Wireframe | Baricéntrico | Scanline |
|---|---|---|
| ![wireframe](images/wireframe.png) | ![barycentric](images/barycentric.png) | ![scanline](images/scanline.png) |

Se dibujan los mismos cuatro triángulos en los tres modos (el último queda parcialmente fuera del framebuffer de 800x600). Los dos rellenos generan **112663 fragmentos cada uno** y las posiciones son idénticas.

## Comprobaciones (`cargo test --release`)

Se comparan las posiciones `(x, y)` generadas por ambos algoritmos (no solo la imagen), y además que no haya duplicados:

- triángulo pequeño y grande
- borde horizontal y borde vertical
- mismos vértices en orden inverso (mismo resultado)
- parcialmente fuera del framebuffer (ningún fragmento fuera de los límites)
- tres vértices colineales y tres vértices iguales (0 fragmentos)
- cuadrado dividido en dos triángulos: 2500 fragmentos exactos, sin huecos ni duplicados
- 20000 triángulos pseudoaleatorios (la mitad con vértices enteros, donde ocurren los empates en bordes)

Resultado: los 7 tests pasan, sin diferencias entre algoritmos.

## Mediciones

`cargo run --release -- --measure`: 200 triángulos fijos (50 grandes y 150 pequeños) en 800x600. Cada medición son 100 ejecuciones de `rasterize` (incluye crear y almacenar los fragmentos; no incluye ventana ni textura), repetida 11 veces; se reporta la mediana. Se usa `black_box` en la entrada y en el resultado.

| Algoritmo | Fragmentos por ejecución | Mediana de 100 ejecuciones |
|---|---|---|
| Baricéntrico | 268610 | 397.89 ms |
| Scanline | 268610 | 127.47 ms |

Ambos producen el mismo número de fragmentos, así que la diferencia de tiempo no se debe a que uno omita trabajo. Los tiempos dependen de la máquina; la proporción (~3x) fue estable entre ejecuciones.

## Perfil

Archivo: [`flamegraph.svg`](flamegraph.svg), generado con `perf record -g` sobre `--benchmark-triangles` (500 repeticiones de ambos algoritmos). Para que cada algoritmo aparezca por separado, `triangle_barycentric` y `triangle_scanline` tienen `#[inline(never)]`.

Porcentaje de muestras (sin hijos):

- `triangle_barycentric`: ~67 %
- `triangle_scanline`: ~15 %
- `realloc`/copia del `Vec` (crecimiento al hacer `push`): ~15 %

## Explicación de las diferencias

El resultado es el mismo, pero el trabajo es distinto. El baricéntrico evalúa tres funciones de arista en **cada** píxel del rectángulo envolvente; en un triángulo, aproximadamente la mitad de ese rectángulo queda fuera, y en triángulos delgados o diagonales es peor. El scanline hace un cálculo geométrico por fila (intersecciones con las aristas) y luego solo genera fragmentos dentro del tramo, sin comprobar píxel por píxel. Por eso casi todo el ancho del flamegraph del baricéntrico es la propia función de rasterización (cálculos geométricos), mientras que en el scanline la geometría casi no aparece y el tiempo se reparte en generar fragmentos.

En ambos casos el crecimiento del `Vec` (`realloc`) ocupa una parte visible del perfil. Una mejora posible sería reservar capacidad por adelantado (por ejemplo, con `Vec::with_capacity` usando una estimación del área), pero no se aplicó para conservar la comparación original. La ventaja del baricéntrico es que ya entrega los pesos `w0, w1, w2` de cada píxel, útiles para interpolar atributos más adelante.

En los bordes no hubo diferencias entre implementaciones porque ambas usan la misma regla top-left: el baricéntrico la aplica cuando una función de arista es exactamente 0, y el scanline mediante los intervalos semiabiertos descritos arriba.
