# Auditoría de Licencias de Terceros (THIRD_PARTY_LICENSES.md)

Este documento contiene la auditoría legal y de licencias de los proyectos, especificaciones y bibliotecas investigadas durante el **Milestone 0** del proyecto **VB6 Open Compiler x64**, de acuerdo con los principios *clean-room* e independientes definidos en el Plan Maestro.

---

## Resumen de la Matriz de Licencias

| Proyecto / Componente | Autor / Organización | Lenguaje | Licencia | Estado de Integración |
| :--- | :--- | :--- | :--- | :--- |
| **VB6Parse** (`scriptandcompile/vb6`) | ScriptAndCompile | Rust | **MIT** | **Apto para integración / adaptación** |
| **ProLeap VB6 Parser** (`uwol/proleap-vb6-parser`) | Ulrich Wolffgang | Java / ANTLR4 | **MIT** | **Apto como referencia semántica / metamodelo** |
| **ANTLR grammars-v4 (vb6)** | ANTLR Community | ANTLR4 | **BSD-3-Clause** | **Apto como referencia gramatical** |
| **LLVM / LLD** | LLVM Foundation | C++ | **Apache-2.0 with LLVM Exception** | **Apto para backend y enlazador** |
| **OpenMSVBVM** (`cocus/openmsvbvm`) | Santiago Hormazabal | C++ | **Sin licencia explícita (Copyright exclusivo)** | **SOLO REFERENCIA DE FIRMAS PÚBLICAS. Prohibido copiar código.** |
| **RAD Basic** | Carles Royan | C# / C / Clang | **Propietario / Comercial cerrado** | **SOLO REFERENCIA CONCEPTUAL. Prohibido copiar código.** |
| **Microsoft VB6 / MSVBVM60.DLL** | Microsoft Corporation | C / C++ / Asm | **Propietario cerrado** | **PROHIBIDO. Clean-room estricto.** |

---

## Análisis Detallado por Componente

### 1. VB6Parse (`projects/vb6parse` en `scriptandcompile/vb6`)
- **Repositorio**: `https://github.com/scriptandcompile/vb6/tree/master/projects/vb6parse`
- **Crate**: `vb6parse` (versión 1.2.4 en crates.io)
- **Licencia**: **MIT** (declarada en `Cargo.toml` a nivel de workspace y crate).
- **Compatibilidad con nuestro proyecto**: **Total**. La licencia MIT es sumamente permisiva y permite uso comercial, modificación, distribución y sublicenciamiento.
- **Valor técnico**:
  - Implementado 100% en Rust.
  - Utiliza `rowan` para crear árboles de sintaxis concreta (CST) resistentes a fallos sintácticos y con preservación total de tokens (trivia, espacios, comentarios).
  - Parsea archivos `.vbp` (proyectos), `.frm` (formularios jerárquicos), `.frx` (recursos binarios empaquetados), `.bas` (módulos de código), `.cls` (módulos de clase) y controles ActiveX (`.ctl`).
- **Dictamen**: Se adopta como base fundamental o dependencia de primer orden para el cargador de proyectos y frontend sintáctico en Rust.

---

### 2. ProLeap VB6 Parser (`uwol/proleap-vb6-parser`)
- **Repositorio**: `https://github.com/uwol/proleap-vb6-parser`
- **Licencia**: **MIT**
- **Compatibilidad con nuestro proyecto**: **Total** a nivel de licencias.
- **Valor técnico**:
  - Proporciona un modelo completo de **ASG (Abstract Semantic Graph)**.
  - Modela con precisión declaraciones de procedimientos (`Sub`, `Function`, `Property Get/Let/Set`), ámbitos léxicos, visibilidad (`Public`, `Private`, `Friend`), tipos definidos por el usuario (UDT), directivas condicionales (`#If...#Else`) e inferencia de tipos asignados.
- **Dictamen**: Se utiliza como referencia arquitectónica para la estructuración de la tabla de símbolos y el analizador semántico en Rust.

---

### 3. ANTLR grammars-v4 (`vb6`)
- **Repositorio**: `https://github.com/antlr/grammars-v4/tree/master/vb6`
- **Licencia**: **BSD-3-Clause**
- **Compatibilidad**: Compatible con MIT y Apache-2.0.
- **Valor técnico**:
  - Gramática formal completa en archivos `VisualBasic6Lexer.g4` y `VisualBasic6Parser.g4`.
  - Excelente catálogo de reglas de producción gramaticales para expresiones, precedencia de operadores, palabras reservadas contextuales y llamadas de subrutinas con o sin paréntesis (regla canónica de VB6).
- **Dictamen**: Referencia para contrastar la cobertura sintáctica del lexer/parser del compilador.

---

### 4. OpenMSVBVM (`cocus/openmsvbvm`)
- **Repositorio**: `https://github.com/cocus/openmsvbvm`
- **Licencia**: **Sin licencia libre explícita** en el repositorio (sujeto al copyright reservado del autor).
- **Riesgo legal**: **Alto** si se copia código directamente.
- **Dictamen Clean-Room**:
  - **No copiar ni adaptar líneas de código**.
  - Utilizar únicamente la lista pública de símbolos exportados (`Source.def`) y la semántica descrita en la documentación de Microsoft (MSDN) para diseñar desde cero el nuevo runtime `VB6Runtime-x64.dll`.
  - Implementación independiente, reproducible y limpia con pruebas de caja negra.

---

### 5. RAD Basic
- **Sitio oficial**: `https://www.radbasic.dev/`
- **Licencia**: **Comercial / Propietaria**.
- **Dictamen**:
  - No decompilar ni analizar binarios de RAD Basic.
  - Sirve únicamente como confirmación de viabilidad en el mercado de que una implementación independiente de VB6 con backend nativo es posible.

---

### 6. LLVM y LLD
- **Licencia**: **Apache-2.0 with LLVM Exceptions**
- **Compatibilidad**: Compatible con la distribución binaria y estática en proyectos comerciales y de código abierto sin contaminar licencias secundarias.
- **Valor técnico**:
  - Soporte completo para emitir código objeto para `x86_64-pc-windows-msvc`.
  - `lld-link` implementa el estándar PE/COFF de Microsoft, generando cabeceras `PE32+`, tablas de exportación/importación y depuración CodeView/PDB nativas.
- **Dictamen**: Seleccionado como backend nativo principal.

---

## Política de Licencia de VB6 Open Compiler x64

El código desarrollado en este repositorio se publica bajo la **Licencia MIT**, garantizando la máxima libertad, adopción por la comunidad y compatibilidad con las directrices de software libre de la OSI.
