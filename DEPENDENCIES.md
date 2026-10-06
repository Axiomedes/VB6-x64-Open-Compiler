# Gestión de Dependencias (DEPENDENCIES.md)

Este documento detalla la cadena de herramientas (*toolchain*), dependencias externas, bibliotecas del sistema y estrategias de enlace para el compilador **VB6 Open Compiler x64** y su runtime **VB6Runtime-x64**.

---

## 1. Cadena de Herramientas del Sistema (Host Toolchain)

El compilador está diseñado para construirse de manera reproducible en Windows 10/11 x64:

| Herramienta | Versión mínima | Versión detectada en el entorno | Propósito |
| :--- | :--- | :--- | :--- |
| **Rust / Cargo** | 1.80+ (Edición 2021/2024) | `rustc 1.99.0` / `cargo 1.99.0` | Lenguaje principal del compilador (`vb6c`), frontend, AST, semántica, IR y CLI |
| **MSVC C/C++** | MSVC v143+ (VS 2022/2026) | `MSVC 14.50.35717` | Compilación del runtime nativo, ABI de Windows x64, SEH y soporte COM |
| **Windows SDK** | 10.0.19041+ | `10.0.26100.0` | Cabeceras Win32 (`windows.h`, `oleauto.h`), bibliotecas de importación (`kernel32.lib`, `user32.lib`, `ole32.lib`, `oleaut32.lib`) |
| **Enlazador (Linker)** | MSVC `link.exe` / `lld-link` | `link.exe` (MSVC Build Tools) | Generación final de ejecutables PE32+ (x86-64) y bibliotecas dinámicas |
| **Git** | 2.30+ | `git 2.55.0` | Control de versiones y trazabilidad de cambios |
| **Python** | 3.10+ | `Python 3.14.3` | Orquestación de golden tests y automatización de validaciones comparativas |
| **VB6 SP6 (Host)** | Visual Studio 98 SP6 | `VB98\VB6.EXE` | Entorno de referencia para pruebas de caja negra y *golden tests* automatizados |

---

## 2. Dependencias del Compilador (Rust Crates)

El proyecto utiliza dependencias del ecosistema Rust seleccionadas por su madurez, rendimiento y seguridad:

### 2.1. Frontend y Análisis Sintáctico
- **`vb6parse`** (`1.2.4`): Motor de análisis sintáctico de código VB6, formularios (`.frm`), módulos (`.bas`), clases (`.cls`), proyectos (`.vbp`) y recursos (`.frx`).
- **`rowan`** (`0.17`): Librería de árbol de sintaxis concreta (*lossless Concrete Syntax Tree*), idéntica a la utilizada en `rust-analyzer`, tolerante a errores sintácticos y con preservación total de tokens.
- **`encoding_rs`** (`0.8`): Soporte robusto para la decodificación de archivos VB6 codificados en ANSI/Windows-1252 a UTF-8/UTF-16.

### 2.2. CLI y Diagnósticos
- **`clap`** (`4.5+`, con soporte `derive`): Parser de argumentos de línea de comandos para el ejecutable `vb6c`.
- **`ariadne`** (`0.4+`): Generación de mensajes de error y diagnósticos con resaltado de código fuente, posición de columnas e indicaciones contextuales de alta legibilidad.
- **`colored`** (`2.1+`): Formateo de salida en consola interactiva.

### 2.3. Estructuras de Datos y Utilidades
- **`thiserror`** (`2.0+`): Definición ergonómica de errores tipados en las capas del compilador.
- **`anyhow`** (`1.0+`): Manejo de errores en la capa CLI y orquestación de tareas de build.
- **`serde`** / **`serde_json`** (`1.0+`): Serialización y deserialización del modelo de proyectos, configuración y volcados de AST/IR.

---

## 3. Dependencias del Runtime x64

El runtime `VB6Runtime-x64.dll` debe ser autónomo y no depender del antiguo `MSVBVM60.DLL`:

### 3.1. APIs del Sistema Operativo Windows
- **Kernel32**: Gestión de memoria (`VirtualAlloc`, `HeapAlloc`), hilos, temporización y cadenas locales.
- **User32**: Creación y gestión de ventanas de formularios (`CreateWindowExW`, `DefWindowProcW`, `MessageBoxW`, bombeo de mensajes de eventos).
- **Gdi32**: Renderizado básico de controles gráficos primitivos.
- **Ole32 & OleAut32**:
  - `SysAllocString`, `SysFreeString`, `SysStringLen` (BSTR nativo de Windows x64).
  - `VariantInit`, `VariantClear`, `VariantCopy`, `VariantChangeType` (gestión de `VARIANT`).
  - `SafeArrayCreate`, `SafeArrayAccessData` (gestión de arreglos multidimensionales compatibles con COM).
  - `CoCreateInstance`, `IUnknown`, `IDispatch` (soporte de objetos y automatización OLE).

---

## 4. Estrategia de Backend y Generación de Código

1. **Frontend**: Genera el AST y el ASG semántico en memoria.
2. **VB6 IR**: Traduce el AST a una representación intermedia independiente del procesador (*Three-Address Code* tipado).
3. **Generación de Código Nativo**:
   - Fase A (Inmediata y reproducible): Emisor de código C nativo / LLVM IR compilable con `clang` o `cl.exe` + `link.exe` para asegurar binarios PE32+ de inmediato sin bloqueos de toolchain.
   - Fase B (Directa): Integración con LLVM (`inkwell` / direct LLVM C API) para generación directa de archivos `.obj` COFF y enlazado con `lld-link` / `link.exe`.
