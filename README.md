# VB6 Open Compiler x64

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Windows x64](https://img.shields.io/badge/Platform-Windows%20x64%20(PE32%2B)-green.svg)](COMPATIBILITY.md)
[![Architecture: Clean Room](https://img.shields.io/badge/Architecture-Clean--Room-brightgreen.svg)](ARCHITECTURE.md)

Implementación independiente, libre, gratuita y de código abierto del lenguaje Microsoft Visual Basic 6.0, capaz de compilar proyectos clásicos existentes y generar ejecutables y bibliotecas nativas de Windows de 64 bits (`PE32+`), procurando que el código fuente existente requiera cero o mínimas modificaciones.

---

## 🎯 Objetivos Principales

1. **Compilación Nativa a 64 Bits**: Producir ejecutables nativos x86-64 (`IMAGE_FILE_MACHINE_AMD64`) a partir de archivos de proyecto `.vbp`, formularios `.frm`, módulos `.bas`, clases `.cls` y recursos `.frx`.
2. **Implementación de Salas Limpias (Clean-Room)**: Ninguna dependencia de código cerrado ni uso de decompilaciones o filtraciones propietarias. Compatibilidad basada estrictamente en comportamiento observable y estándares públicos.
3. **Cero o Mínimas Modificaciones**: Permitir que el código VB6 clásico funcione de manera transparente sin necesidad de traducción a C#, VB.NET u otros lenguajes.
4. **Cadena de Herramientas Moderna**: Frontend y compilador desarrollados en Rust para máxima confiabilidad, velocidad y seguridad en memoria; backend basado en representaciones intermedias estructuradas y enlazado PE32+.

---

## 🏗️ Arquitectura General

```text
       Proyecto VB6 (.vbp, .frm, .bas, .cls)
                         │
                         ▼
                  Project Loader
                         │
                         ▼
                  Lexer & Parser
                         │
                         ▼
             AST & Abstract Semantic Graph
                         │
                         ▼
                   Semantic Model
                         │
                         ▼
                      VB6 IR
                         │
                         ▼
                  Backend x64 / LLVM
                         │
                         ▼
                  PE32+ Linker
                         │
                         ▼
               Proyecto.exe (x64 Nativo)
                         │
                         ▼
                VB6Runtime-x64.dll
```

Consulte [ARCHITECTURE.md](ARCHITECTURE.md) para un desglose completo de cada componente.

---

## 📋 Documentación de Referencia

- [ARCHITECTURE.md](ARCHITECTURE.md): Diseño detallado del sistema, flujo de compilación y subsistemas.
- [DEPENDENCIES.md](DEPENDENCIES.md): Cadena de herramientas del sistema, dependencias y bibliotecas.
- [COMPATIBILITY.md](COMPATIBILITY.md): Matriz de niveles de compatibilidad (Nivel 0 al 8) y especificación del sistema de tipos.
- [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md): Auditoría legal de las tecnologías evaluadas.
- [CONTRIBUTING.md](CONTRIBUTING.md): Guía de desarrollo y políticas *clean-room*.

---

## 🚀 Uso Rápido (Línea de Comandos)

```powershell
# Compilar un proyecto clásico directamente a x64
vb6c Proyecto.vbp --target=x64

# Compilar un archivo de módulo individual
vb6c Module1.bas --target=x64 --output=Hello.exe

# Modo release con optimizaciones nativas
vb6c Proyecto.vbp --target=x64 --release
```

---

## 🧪 Pruebas y Validación (*Golden Tests*)

Para garantizar que no haya desviaciones de comportamiento respecto al entorno original, las pruebas se validan de forma cruzada contra **VB6 SP6**:

```powershell
# Ejecutar suite de pruebas de compatibilidad
cargo test
```

Consulte [tests/](tests/) para el catálogo de pruebas automatizadas.

---

## 📜 Licencia

Distribuido bajo la [Licencia MIT](LICENSE).
