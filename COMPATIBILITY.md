# Matriz de Compatibilidad y Especificación de Comportamiento (COMPATIBILITY.md)

Este documento define la matriz objetiva de niveles de compatibilidad, la semántica observable del lenguaje Visual Basic 6.0 y las consideraciones críticas para la arquitectura **Windows x64 (PE32+)**.

---

## 1. Niveles Objetivos de Compatibilidad

De acuerdo con el Plan Maestro, el estado del compilador se evalúa mediante métricas verificables en lugar de afirmaciones cualitativas:

| Nivel | Nombre | Criterio de Éxito | Estado |
| :--- | :--- | :--- | :--- |
| **Nivel 0** | **Parser y AST** | Análisis sintáctico sin errores de proyectos (`.vbp`), módulos (`.bas`), clases (`.cls`) y formularios (`.frm`). Preservación de trivia y reporte de errores sintácticos con posición exacta. | **En Progreso (Milestone 1)** |
| **Nivel 1** | **Compilador Básico** | Compilación de un programa básico (`Sub Main()` con `MsgBox` / `Debug.Print`) generando un ejecutable nativo PE32+ x64 ejecutable sin dependencias externas. | **Objetivo Inmediato (Milestone 5)** |
| **Nivel 2** | **Semántica del Lenguaje** | Sistema de tipos completo, operadores aritméticos/lógicos/bit a bit, estructuras de control (`If`, `Select Case`, `For...Next`, `Do...Loop`, `While...Wend`), `ByRef` por defecto, paso de argumentos y resolución de ámbitos. | Planificado |
| **Nivel 3** | **Runtime y Tipos Centrales** | Tipos `Variant`, coerciones (`CInt`, `CLng`, `CStr`, etc.), funciones intrínsecas de cadena (`Left`, `Mid`, `Right`, `InStr`), matemáticas, fecha y arrays dinámicos (`ReDim Preserve`). Manejo de errores estructurado (`On Error GoTo`, `Resume`). | Planificado |
| **Nivel 4** | **Formularios y Controles** | Carga y renderizado de `.frm` y recursos `.frx`. Controles intrínsecos: `Form`, `CommandButton`, `TextBox`, `Label`, `CheckBox`, `OptionButton`, `Timer`, `Frame`. Ciclo de vida y bombeo de eventos Win32. | Planificado |
| **Nivel 5** | **COM y OLE Automation** | `IUnknown`, `IDispatch`, enlace tardío (*late binding* con `CreateObject`), enlace temprano (*early binding* con `New`), `SAFEARRAY`, `BSTR` y variantes COM. | Planificado |
| **Nivel 6** | **Acceso a Datos y WinAPI** | Declaraciones `Declare PtrSafe` con compatibilidad para punteros de 64 bits (`LongPtr`), interoperabilidad con Win32 y ADO/OLE DB nativo de 64 bits. | Planificado |
| **Nivel 7** | **Componentes Heredados (Bridge x86)** | Puente IPC/COM fuera de proceso (*out-of-process COM bridge*) para comunicar el proceso x64 con componentes ActiveX/OCX de 32 bits heredados. | Planificado |
| **Nivel 8** | **Aplicaciones Reales de Gran Escala** | Compilación y ejecución exitosa de aplicaciones comerciales VB6 históricas complejas con cero o mínimas modificaciones. | Planificado |

---

## 2. Sistema de Tipos: VB6 vs. VB6 x64

Para garantizar que el código VB6 clásico funcione sin alteraciones inesperadas, el compilador mantiene el ancho semántico de los tipos estándar y añade soporte para punteros de 64 bits:

| Tipo VB6 | Tamaño Win32 (x86) | Tamaño Win64 (x64) | Rango / Descripción |
| :--- | :--- | :--- | :--- |
| **Byte** | 1 byte (8 bits) | 1 byte (8 bits) | 0 a 255 (entero sin signo) |
| **Boolean** | 2 bytes (16 bits) | 2 bytes (16 bits) | `-1` para `True`, `0` para `False` (lógica negada canónica VB6) |
| **Integer** | 2 bytes (16 bits) | 2 bytes (16 bits) | -32,768 a 32,767 (entero con signo) |
| **Long** | 4 bytes (32 bits) | 4 bytes (32 bits) | -2,147,483,648 a 2,147,483,647 |
| **LongPtr** *(Extensión x64)* | 4 bytes (en x86) | **8 bytes (en x64)** | Tipo condicional para punteros y descriptores de Windows (`HWND`, `HANDLE`, punteros de memoria). |
| **LongLong** *(Extensión x64)* | N/A en VB6 clásico | **8 bytes (64 bits)** | Entero de 64 bits con signo (-9,223,372,036,854,775,808 a 9,223,372,036,854,775,807). |
| **Single** | 4 bytes (32 bits) | 4 bytes (32 bits) | Coma flotante IEEE 754 de precisión simple |
| **Double** | 8 bytes (64 bits) | 8 bytes (64 bits) | Coma flotante IEEE 754 de precisión doble |
| **Currency** | 8 bytes (64 bits) | 8 bytes (64 bits) | Entero escalado con 4 decimales implícitos (división exacta entre 10,000) |
| **Date** | 8 bytes (64 bits) | 8 bytes (64 bits) | Número flotante con fecha base 30 de diciembre de 1899 |
| **String** | 4 bytes (puntero BSTR) | **8 bytes (puntero BSTR)** | Puntero a cadena UTF-16 con prefijo de longitud de 4 bytes antes del puntero |
| **Variant** | 16 bytes | **24 bytes** | En Windows x64, la estructura `VARIANT` de OLE Automation crece a 24 bytes debido a la alineación de punteros de 64 bits (`vt` de 2 bytes, 6 bytes de relleno y 16 bytes de unión de datos). |
| **Object** | 4 bytes (puntero COM) | **8 bytes (puntero COM)** | Puntero a interfaz `IUnknown` / `IDispatch` |

> [!IMPORTANT]
> En código VB6 clásico, los programadores a menudo usaban `Long` para almacenar identificadores de ventanas (`HWND`) o punteros (`lParam`, `wParam`). En x64, un `Long` de 32 bits truncaría un puntero de 64 bits. El compilador advertirá cuando un valor `Long` interactúe con APIs de Win32 y facilitará la compatibilidad mediante la directiva `PtrSafe` y el tipo `LongPtr` (idéntico al estándar de VBA 7 de 64 bits).

---

## 3. Comportamientos Observables Críticos

### 3.1. Evaluación y Paso de Parámetros (`ByRef` por defecto)
- En VB6, todos los parámetros de procedimientos (`Sub` o `Function`) son pasados por omisión como **`ByRef`** a menos que se indique explícitamente `ByVal`.
- Si un parámetro `ByRef` recibe una expresión compleja o una variable entre paréntesis extra (`Call Test((X))`), se fuerza la creación de un temporal y se comporta como `ByVal`.

### 3.2. Operadores Lógicos y Bit a Bit
- Los operadores `And`, `Or`, `Xor`, `Not`, `Eqv`, `Imp` son operadores **a nivel de bits** (*bitwise*), no de cortocircuito booleano. Por ejemplo:
  `If FuncA() And FuncB() Then` evalúa **ambas funciones obligatoriamente**.

### 3.3. Arreglos y ReDim
- La base por defecto es `0` a menos que exista `Option Base 1`.
- Índices explícitos: `Dim A(1 To 10)` tiene `LBound = 1` y `UBound = 10`.
- `ReDim Preserve` en arreglos multidimensionales solo puede alterar el límite superior de la **última dimensión**.

### 3.4. Manejo de Errores Clásico
- `On Error GoTo <Etiqueta>` activa un manejador de excepciones de ámbito local.
- `On Error Resume Next` suprime la detención y actualiza el objeto `Err`.
- `On Error GoTo 0` desactiva cualquier manejador activo en el procedimiento actual.
- Al generar código nativo x64, el manejador de errores de VB6 se integrará con las tablas de desenrollado de pila (*stack unwind tables*) y SEH (Structured Exception Handling) de Windows x64.

---

## 4. Suite de Pruebas Comparativas (*Golden Tests*)

Para validar la compatibilidad absoluta, se implementa una infraestructura de pruebas automatizadas:

1. **Ubicación**: `tests/compatibility/` y `tests/golden/`.
2. **Procedimiento de verificación**:
   - Se compila y ejecuta el test con el compilador de referencia **VB6 SP6** (`VB6.EXE`).
   - Se captura la salida estándar, salida de error, código de salida y archivos generados.
   - Se compila y ejecuta el mismo caso de prueba con **`vb6c`** en x64.
   - Un script de comparación automática valida que la salida observable sea idéntica.
