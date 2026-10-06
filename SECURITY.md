# Política de Seguridad - VB6 Open Compiler x64

## Reporte de Vulnerabilidades

La seguridad de las aplicaciones generadas por el compilador y de la cadena de herramientas en sí misma es de vital importancia. Si descubres una vulnerabilidad de seguridad en el compilador, su runtime o en las herramientas asociadas, por favor sigue este procedimiento:

1. **No abras un issue público** para vulnerabilidades de día cero o fallos que comprometan la seguridad de ejecución.
2. Envía un reporte detallado a los mantenedores principales del proyecto por correo privado o a través del canal de divulgación coordinada.
3. Incluye un ejemplo reproducible mínimo (PoC), la versión afectada y los impactos observados (ej. desbordamiento de búfer en runtime, ejecución arbitraria de código durante la compilación, etc.).

## Alcance de Seguridad

- **Compilador (`vb6c`)**: Debe validar de manera segura los archivos de entrada (.vbp, .frm, .bas, .cls, .frx) evitando ataques de deserialización o desbordamientos al parsear recursos binarios.
- **Runtime (`VB6Runtime-x64`)**: Debe implementar salvaguardas modernas de memoria en x64 (compatibilidad con ASLR, DEP/NX, control de límites en arrays seguros y manejo protegido de punteros COM/BSTR).
