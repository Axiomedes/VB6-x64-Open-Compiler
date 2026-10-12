# VB6 Open Compiler x64 (`vb6c`)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Platform: Windows x64](https://img.shields.io/badge/Platform-Windows%20x64%20(PE32%2B)-green.svg)](COMPATIBILITY.md)
[![Architecture: Clean Room](https://img.shields.io/badge/Architecture-Clean--Room-brightgreen.svg)](ARCHITECTURE.md)
[![Tests: 100% Passing](https://img.shields.io/badge/Tests-100%25%20Passing-success.svg)](tests/)

Compilador nativo, independiente, libre y de código abierto para Microsoft Visual Basic 6.0 (VB6), diseñado para compilar proyectos clásicos existentes y generar binarios nativos de Windows de 64 bits (`PE32+`), con cero dependencias de bibliotecas propietarias de 32 bits (`msvbvm60.dll`) y garantizando compatibilidad observable con el comportamiento histórico de VB6 SP6.

---

## 🎯 Objetivos y Principios del Proyecto

1. **Generación Nativa Windows x64 (`IMAGE_FILE_MACHINE_AMD64`)**:
   - Produce binarios ejecutables PE32+ de 64 bits directos a partir de proyectos `.vbp`, módulos `.bas`, clases `.cls`, formularios `.frm` y recursos `.frx`.
2. **Implementación de Sala Limpia (*Clean-Room*)**:
   - Cero uso de decompilaciones, ingeniería inversa de binarios protegidos o filtraciones.
   - Compatibilidad deducida rigurosamente a partir de documentación pública, especificaciones COM TypeLib IDL (`VB6.OLB`) y pruebas comparativas de comportamiento observable (*Golden Tests*).
3. **Cero o Mínimas Modificaciones de Código**:
   - Permite compilar bases de código VB6 existentes preservando la semántica clásica de paso por referencia (`ByRef` por defecto), operadores a nivel de bits, manejo de errores estructurado (`On Error GoTo` / `Resume Next`) y modelo de eventos.
4. **Cadena de Herramientas de Alto Rendimiento**:
   - Frontend y análisis semántico desarrollados en Rust para máxima seguridad en memoria y velocidad.
   - Representación Intermedia (IR) estructurada en bloques básicos (SSA-friendly).
   - Backend nativo con generación de código LLVM IR y ensamblado a objetos COFF x64.
   - Enlazador PE32+ nativo con resolución de símbolos y tabla de importaciones Win32.
   - Runtime en C de sala limpia compilado para arquitectura x86-64.

---

## 🏗️ Arquitectura del Compilador

```text
       Proyecto Clásico VB6 (.vbp, .bas, .cls, .frm)
                            │
                            ▼
           Project Loader & Encoding Detector
              (ANSI / Windows-1252 / UTF-8)
                            │
                            ▼
              Lexer & Parser Recursivo (AST)
                            │
                            ▼
                 Análisis Semántico &
             Tabla de Símbolos Multi-Ámbito
                            │
                            ▼
               VB6 Intermediate Representation
                   (Bloques Básicos en IR)
                            │
                            ▼
                 Backend LLVM / Emitter x64
                            │
                            ▼
                Generación COFF Object (.obj)
                            │
                            ▼
              Linker PE32+ (Kernel32, User32, GDI32)
                            │
                            ▼
               Ejecutable Nativo x64 (.exe)
```

---

## 🚀 Estado Actual del Desarrollo

El compilador ha completado con éxito los **Hitos 0 al 16** con **100% de pruebas unitarias y de integración pasando sin regresiones**:

- :white_check_mark: **Frontend Completo**: Soporte total de sintaxis VB6, preservación de spans y diagnóstico preciso de errores sintácticos y semánticos.
- :white_check_mark: **Sistema de Tipos x64**: Enteros (`Byte`, `Integer`, `Long`, `LongLong`), punto flotante (`Single`, `Double`), moneda (`Currency`), fechas (`Date`), cadenas (`String` BSTR) y variantes (`Variant` de 24 bytes alineados para 64 bits).
- :white_check_mark: **Estructuras de Control**: `If...Then...ElseIf...Else`, `Select Case`, `For...Next` (pasos positivos/negativos), `Do...Loop` (`While`/`Until` al inicio y al final), `While...Wend`, `Exit Sub/Function/For/Do`.
- :white_check_mark: **Arreglos Multidimensionales**: Arreglos estáticos y dinámicos (`ReDim`, `ReDim Preserve`), límites arbitrarios (`LBound` a `UBound`, índices negativos).
- :white_check_mark: **Cadenas de Texto y Funciones Intrínsecas**: Operadores `&` y `+`, `Len`, `Left`, `Right`, `Mid`, `InStr`, `Trim`, `LTrim`, `RTrim`, `UCase`, `LCase`, `Space`, `String`, `Replace`, `Chr`, `Asc`, `Val`, `Str`.
- :white_check_mark: **Tipos Variant y Coerción Dinámica**: Evaluación de expresiones polimórficas, inspección (`IsNumeric`, `IsEmpty`, `IsNull`) y funciones de conversión explícita (`CInt`, `CLng`, `CDbl`, `CStr`, `CBool`, `CByte`, `CCur`, `CDate`).
- :white_check_mark: **UDTs y Enums**: Estructuras de datos (`Type...End Type`), cadenas de longitud fija (`String * N`), enumeraciones (`Enum...End Enum`), asignación por copia de estructuras y bloques `With...End With`.
- :white_check_mark: **Clases y Modelo de Objetos (`.cls`)**: Instanciación (`New`, `Dim ... As New`), propiedades (`Property Get`, `Property Let`, `Property Set`), métodos de clase, operador `Me`, constructores/destructores de ciclo de vida (`Class_Initialize`, `Class_Terminate`), operadores de identidad (`Is Nothing`, `Is`) y conteo de referencias automático (*Reference Counting* con `retain`/`release`).
- :white_check_mark: **Formularios y Controles Win32 (`.frm`)**:
  - Desazucarado de archivos `.frm` a definiciones de Clases VB6 fuertemente tipadas.
  - Instancia global por defecto del formulario (`Public Form1 As Form1`) bajo el modelo canónico `VB_PredeclaredId = True`.
  - Controles intrínsecos implementados según especificación TypeLib IDL de `VB6.OLB`: `CommandButton`, `TextBox`, `Label`, `CheckBox`, `OptionButton`, `Frame`, `Timer` y `Form`.
  - Subsistema de interfaz gráfica Win32 x64 con registro de ventana `VB6_Form_Class`, despacho de eventos (`Click`, `Change`, `Timer`, `Load`, `Unload`) y bombeo no bloqueante con `DoEvents`.

---

## 🗺️ Hoja de Ruta (Roadmap)

### Hitos Completados :white_check_mark:

| Hito | Nombre | Descripción | Estado |
| :---: | :--- | :--- | :---: |
| **0** | **Arquitectura y Gobernanza** | Especificación técnica, gobernanza clean-room, licencia GPLv3 y documentación. | **Completado** |
| **1** | **Project Loader** | Carga y parseo de `.vbp`, `.bas`, `.cls`, `.frm`, `.frx`, `.ctl`, `.pag`. | **Completado** |
| **2** | **Lexer y Parser AST** | Gramática completa de VB6, árbol sintáctico abstracto fuertemente tipado. | **Completado** |
| **3** | **Modelo Semántico** | Tabla de símbolos multi-ámbito, validación de tipos, ámbitos de visibilidad. | **Completado** |
| **4** | **Representación Intermedia (IR)** | Generación de bloques básicos, SSA-friendly, flujo de control estructurado. | **Completado** |
| **5** | **Sistema de Tipos x64** | Tamaños y alineaciones x64, Variant de 24 bytes, enteros y punto flotante. | **Completado** |
| **6** | **Backend LLVM x64** | Emisión de LLVM IR x64, invocación de toolchain y generación COFF `.obj`. | **Completado** |
| **7 & 8** | **Linker PE32+ & Hola Mundo** | Enlazado nativo PE32+ de 64 bits y primer binario ejecutable x64 autónomo. | **Completado** |
| **9** | **Variables y Asignaciones** | Variables locales, globales, estáticas (`Static`), inferencia y asignación. | **Completado** |
| **10** | **Estructuras de Control** | `If/Then/Else`, `Select Case`, `For/Next`, `Do/Loop`, `While/Wend`, `Exit`. | **Completado** |
| **11** | **Arreglos Multidimensionales** | Arreglos estáticos y dinámicos, `ReDim`, `ReDim Preserve`, `LBound`, `UBound`. | **Completado** |
| **12** | **Cadenas y Funciones Intrínsecas** | Operadores de concatenación, manipulación de cadenas BSTR y funciones estándar. | **Completado** |
| **13** | **Variantes y Coerción** | Evaluación polimórfica en tiempo de ejecución, funciones de inspección y cast. | **Completado** |
| **14** | **UDTs, Enums y With** | Tipos de usuario (`Type`), enumeraciones (`Enum`), cadenas fijas y `With`. | **Completado** |
| **15** | **Clases y Modelo de Objetos** | Clases (`.cls`), `Property Get/Let/Set`, `New`, ciclo de vida y ref counting. | **Completado** |
| **16** | **Formularios y Controles Win32** | Formularios (`.frm`), controles intrínsecos, eventos Win32 y `DoEvents`. | **Completado** |

---

### Hitos Pendientes :hammer_and_wrench:

| Hito | Nombre | Alcance y Entregables | Prioridad |
| :---: | :--- | :--- | :---: |
| **17** | **User Controls y Property Pages** | Soporte para controles de usuario (`.ctl`), páginas de propiedades (`.pag`), definición y disparo de eventos personalizados (`Event`, `RaiseEvent`) y persistencia de propiedades en tiempo de diseño. | **Inmediata (Siguiente)** |
| **18** | **Interoperabilidad COM x64** | Implementación nativa de `IUnknown` e `IDispatch`, enlace tardío (*late binding* con `CreateObject`), enlace temprano (*early binding* con TypeLibs `.olb`/`.tlb`), paso de `SAFEARRAY` y variantes COM en 64 bits. | Alta |
| **19** | **I/O de Archivos Clásico** | Declaraciones de entrada/salida clásicas de VB6: `Open`, `Close`, `Print #`, `Write #`, `Input #`, `Get #`, `Put #`, `Seek`, `LOF`, `EOF`, modos `Binary`, `Random` y secuencial. | Alta |
| **20** | **Win32 API y PtrSafe en x64** | Declaraciones `Declare Sub/Function ... Lib ... Alias ... [PtrSafe]`, paso de parámetros por valor/referencia, punteros de función mediante `AddressOf` y soporte de callbacks Win64 nativos. | Alta |
| **21** | **Puente ActiveX 32-bit (IPC Bridge)** | Puente fuera de proceso (*out-of-process COM/IPC bridge*) para permitir que aplicaciones x64 instancien y utilicen controles OCX / DLLs ActiveX de 32 bits heredadas sin recompilar. | Media |
| **22** | **Bibliotecas ActiveX (DLL y EXE x64)** | Compilación de proyectos de tipo `ActiveX DLL` y `ActiveX EXE` a binarios nativos x64 con tablas de clases COM registrables en el Registro de Windows de 64 bits. | Media |
| **23** | **Depuración y Optimizaciones** | Emisión de información de depuración compatible con PDB / DWARF para depuración paso a paso en Visual Studio / WinDbg / LLDB, y pases de optimización (`-O2`, `-O3`, LTO). | Media |
| **24** | **Validación a Gran Escala (Golden Suite)** | Compilación y ejecución exitosa de aplicaciones comerciales VB6 históricas completas con verificación cruzada contra VB6 SP6. | Hito Final |

---

## 💻 Uso de la Línea de Comandos (`vb6c`)

### Compilación Básica

```powershell
# Compilar un proyecto completo (.vbp) con formulario de inicio a x64
vb6c Proyecto.vbp --target=x64

# Compilar un formulario individual (.frm)
vb6c Form1.frm --output=App.exe

# Compilar un módulo estándar (.bas)
vb6c Module1.bas --output=Util.exe

# Compilación optimizada para producción
vb6c Proyecto.vbp --release --output=Bin/App.exe
```

### Opciones de Diagnóstico

```powershell
# Emitir la Representación Intermedia (VB6 IR) generada
vb6c Proyecto.vbp --emit-ir

# Emitir el código LLVM IR x64 generado
vb6c Proyecto.vbp --emit-llvm

# Modo detallado con información de enlazado PE32+
vb6c Proyecto.vbp --verbose
```

---

## 🧪 Ejecución de Pruebas

El repositorio cuenta con una suite completa de pruebas automatizadas que se ejecutan de manera headless:

```powershell
# Ejecutar todas las pruebas del workspace
cargo test --workspace

# Ejecutar únicamente las pruebas del Hito 16 (Formularios y Controles)
cargo test --test milestone_16_forms_controls_tests

# Ejecutar pruebas de modelos de objetos (Hito 15)
cargo test --test milestone_15_classes_tests
```

---

## 📜 Licencia

Este proyecto está licenciado bajo la **Licencia Pública General de GNU Versión 3 (GPLv3)**. Consulte el archivo [LICENSE](LICENSE) para más detalles.
