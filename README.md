# learn-git-tui

Aplicación de terminal para aprender git practicando con git real. Cada ejercicio corre en un sandbox aislado y se verifica mirando el estado del repositorio, no los comandos que tipeaste. Todo el contenido está en inglés y español.

## Qué incluye

- 11 niveles y 156 lecciones, agrupadas por uso: fundamentos, ramas y tags, remotos, deshacer y reescribir, investigar la historia, parches por mail, repos grandes, configuración y seguridad, plumbing, almacenamiento y servidores.
- 153 ejercicios con git real. Los temas internos u obsoletos se explican sin ejercicio.
- Consola integrada con `git` y unos pocos comandos propios (`ls`, `cat`, `cd`, `pwd`, `mkdir`, `touch`, `rm`, `chmod +x`, `echo > / >>`, `edit`, `clear`). No usa una shell, así que anda igual en Windows y Linux.
- Editor integrado: git lo abre para `commit` sin `-m`, `rebase -i`, `mergetool`, etc.
- Pistas graduales, puntos, logros, examen de ubicación y modo libre.
- Progreso guardado entre sesiones.

## Requisitos

- git 2.34 o más nuevo, en el `PATH`.
- Opcionales. Si faltan, la lección correspondiente pasa a ser solo explicación:
  - `ssh-keygen`: lecciones de commits y tags firmados con SSH.
  - `git subtree`: lección de subtree (algunas distros no lo incluyen).
  - git 2.38 o más nuevo: `git diagnose` y `git merge-tree --write-tree`.
- Para compilar: Rust estable con edición 2024 (1.85 o más nuevo).

## Instalación

Desde las releases de GitHub: bajá el binario de tu sistema y ejecutalo.

- Windows x86_64: `learn-git-tui-windows-x86_64.exe`
- Linux x86_64 (estático): `learn-git-tui-linux-x86_64`

Desde el código fuente:

```sh
cargo install --git https://github.com/matiasdante/learn-git-in-a-tui
```

O, dentro de un clon del repositorio:

```sh
cargo install --path .
```

macOS no tiene binario publicado; se instala con `cargo install`.

## Uso

```sh
learn-git-tui            # abre la aplicación
learn-git-tui --version
```

En el primer arranque elegís el idioma y podés rendir un examen de ubicación de 10 preguntas (2 por cada uno de los niveles 1 a 5). Con todo correcto arrancás en el nivel 6; si lo omitís, en el 1. Los niveles anteriores siempre quedan disponibles.

| Tecla | Acción |
| --- | --- |
| `↑` `↓` / `Enter` / `Esc` | moverse, abrir, volver |
| `L` | cambiar de idioma (menús) |
| `F1` | pedir una pista (−25 puntos) |
| `F5` | verificar el ejercicio (cada fallo resta 10 puntos) |
| `F6` | pasar al ejercicio siguiente |
| `Ctrl+R` | reiniciar el ejercicio (en modo libre, reiniciar el sandbox) |
| `Ctrl+D` | cerrar el stdin del comando en curso (para `credential approve`, `mktree`...) |
| `Ctrl+C` | detener el comando en curso o limpiar la línea |
| `Ctrl+S` / `Esc` | en el editor: guardar y cerrar / cancelar |
| `Ctrl+Q` | salir |

Cada ejercicio vale 100 puntos, con un mínimo de 25. Si la verificación falla, la aplicación explica qué faltó y restaura el estado del inicio del ejercicio.

## Dónde guarda datos

Todo está en el directorio de datos del usuario, en la carpeta `learn-git-tui`:

- `progress.json`: idioma, ejercicios hechos, puntos y logros.
- `sandbox/`: el repositorio de práctica, un `HOME` propio y un `gitconfig` propio.

Cada proceso de git corre con `HOME` y `GIT_CONFIG_GLOBAL` apuntando al sandbox, `GIT_CONFIG_NOSYSTEM=1`, `GIT_TERMINAL_PROMPT=0` y fechas fijas (los hashes son estables). Tu `~/.gitconfig` y tus credenciales reales no se leen ni se modifican. `git -C`, `--git-dir` y `--work-tree` solo se aceptan si apuntan dentro del sandbox.

Para empezar de cero, borrá `progress.json`.

## Desarrollo

```sh
cargo test                                  # unitarios, integración y contenido
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

El test de contenido recorre cada ejercicio: el estado inicial sin tocar tiene que fallar los checks y la solución de referencia tiene que pasarlos. También exige que todo texto exista en `en` y `es`. El CI corre todo en Ubuntu y Windows.

Las lecciones son archivos TOML en `content/`, embebidos en el binario. Un ejercicio define el `setup` (acciones del motor, no shell), las `hints`, los `checks` declarativos y una `solution` de referencia. Los checks usan plumbing y formatos de máquina (`status --porcelain=v2`, `rev-parse`, `for-each-ref`...), nunca la salida pensada para humanos.

Cómo funciona el editor integrado: `GIT_EDITOR` y `GIT_SEQUENCE_EDITOR` apuntan al propio ejecutable con el subcomando oculto `__editor <archivo>`. Ese proceso avisa a la interfaz por archivos en el directorio `ipc/` del sandbox y espera la respuesta.

## Releases

Un tag `v*` compila el `.exe` de Windows y el binario estático de Linux (musl) y los publica en Releases.

```sh
git tag -a v1.0.0 -m "v1.0.0"
git push origin v1.0.0
```

## Fuera de alcance

Remotos reales con red, binarios para macOS, salida con colores ANSI, mouse, herramientas de merge externas, actualizador automático y otros idiomas.
