# Directrices de Contribución - VB6 Open Compiler x64

Gracias por tu interés en contribuir a **VB6 Open Compiler x64**. Este proyecto busca proporcionar una implementación independiente, libre, gratuita y moderna del lenguaje Microsoft Visual Basic 6.0 capaz de producir binarios Windows x64 nativos.

---

## 1. Principio Fundamental Clean-Room (Salas Limpias)

La integridad legal y técnica del proyecto es la prioridad máxima:

1. **PROHIBIDO incorporar código propietario de Microsoft**: No se aceptará bajo ninguna circunstancia código fuente descompilado, filtrado o perteneciente a Microsoft Visual Basic 6.0, Microsoft Visual Studio o MSVBVM60.DLL.
2. **Implementación de Comportamiento Observable**: Todas las funciones del compilador y runtime deben basarse en documentación pública oficial (MSDN clásica, especificaciones COM/OLE, Win32 API), tests de caja negra comparativos y especificaciones del lenguaje.
3. **Auditoría de Licencias de Terceros**: Cualquier dependencia externa o código referenciado debe poseer una licencia compatible (MIT, Apache-2.0, BSD) y estar documentado en `THIRD_PARTY_LICENSES.md`.

---

## 2. Flujo de Desarrollo y Calidad

Una característica **no se considera terminada** únicamente porque compile. El criterio de aceptación incluye:

- **Análisis Sintáctico (Parser) Correcto**: Soporte de la sintaxis canónica VB6 y tolerancia a peculiaridades del lenguaje.
- **Modelo Semántico Verificado**: Resolución correcta de ámbitos, enlace de tipos e inferencia de tipos VB6.
- **Generación de Código (IR y Backend)**: Emisión de instrucciones intermedias y código nativo válido.
- **Runtime Compatible**: Preservación de la semántica de error, coerción de variantes, manejo de cadenas BSTR y ciclo de vida de objetos.
- **Tests Automatizados**:
  - Test unitario sintáctico y semántico.
  - Test comparativo (*golden test*) verificable contra VB6 SP6 en entornos donde esté disponible.

---

## 3. Estructura del Código

- **Frontend, AST y Semántica**: Modular, legible, con diagnósticos claros y precisos.
- **VB6 IR**: Representación intermedia agnóstica de la máquina destino, desacoplada de LLVM.
- **Runtime**: Diseñado para soportar x86, x64 nativo y potencialmente ARM64.
- **Tests**: Ubicados en `tests/compatibility/` y `tests/golden/`.

---

## 4. Proceso de Pull Requests

1. Crea una rama descriptiva (`feature/nombre-de-la-funcionalidad` o `fix/descripcion-del-bug`).
2. Agrega los tests de compatibilidad correspondientes antes o junto a la implementación.
3. Asegúrate de que toda la suite de pruebas pase (`cargo test`).
4. Documenta cualquier decisión técnica relevante o comportamiento observable verificado.
