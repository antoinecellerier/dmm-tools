<!-- translated from README.md b3f3a0a3722dd935b8bf521ad26efad3a0c63a18 -->
# dmm-tools

[![CI](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml)
[![Versión](https://img.shields.io/github/v/release/antoinecellerier/dmm-tools)](https://github.com/antoinecellerier/dmm-tools/releases/latest)
[![Licencia: GPL-3.0-or-later](https://img.shields.io/github/license/antoinecellerier/dmm-tools)](LICENSE)

[English](README.md) · **Español** · [Français](README.fr.md) · [Português (Brasil)](README.pt-BR.md) · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

*Esta página es una traducción automática; la versión de referencia es el [README en inglés](README.md). Se agradece la revisión de hablantes nativos: [cómo colaborar](CONTRIBUTING.md#translations) (en inglés).*

Aplicación de escritorio y herramienta de línea de comandos de código abierto para leer, registrar, representar gráficamente y controlar a distancia multímetros digitales (DMM) y pinzas amperimétricas de UNI-T, ZOTEK (ZOYI, ANENG, BSIDE), OWON, Brymen (Minipa), EEVblog y Voltcraft, por USB y Bluetooth, en Linux, Windows, macOS y Raspberry Pi.

Funciona con los UNI-T UT61E+, UT181A, UT161, UT71, UT60BT y UT804, ANENG AN9002 (ZOTEK ZT-300AB), Brymen BM869s, EEVblog 121GW, OWON OW18E y B41T+, Voltcraft VC-890 y [más](#dispositivos-compatibles).

**Contenido:** [Funciones](#funciones) · [Dispositivos compatibles](#dispositivos-compatibles) · [Inicio rápido](#inicio-rápido) · [Documentación](#documentación) · [Comentarios](#comentarios) · [Licencia](#licencia)

![dmm-gui en una sesión de corriente continua en mA: la lectura en vivo, las especificaciones y las estadísticas junto al gráfico, donde el minimapa destaca la secuencia de arranque de un sensor, dos cursores miden su duración y su carga, y unos marcadores nombran sus etapas](assets/gui-wide-layout.es.webp)

## Funciones

Tanto la aplicación de escritorio como la herramienta de línea de comandos:

- Detectan qué multímetro está conectado
- Controlan el multímetro a distancia: cambian el modo y el rango, presionan sus botones
- Guardan las lecturas en CSV, JSON o un archivo de reproducción y las vuelven a abrir
- Calculan estadísticas: mínimo, máximo, media, desviación estándar e integral respecto al tiempo
- Activan una alarma cuando una lectura traspasa un límite
- Aplican por software una escala, un desplazamiento y una unidad propia a las lecturas
- Muestran la interfaz en inglés, chino simplificado, español, francés, japonés o portugués de Brasil, según el idioma del sistema

### Aplicación de escritorio (`dmm-gui`)

La aplicación de escritorio además:

- Muestra la lectura en vivo con la resolución y la exactitud del rango según el manual
- Traza las lecturas en un gráfico en vivo, con cursores, líneas de referencia, una envolvente mín./máx. y un histograma
- Graba sesiones largas, con marcadores y notas en cualquier lectura
- Muestra la lectura ocupando toda la ventana en el modo de lectura grande

[Ayuda de dmm-gui](docs/gui-reference.md) (en inglés): la guía completa de la aplicación de escritorio.

### Herramienta de línea de comandos (`dmm-cli`)

La herramienta de línea de comandos además:

- Transmite las lecturas como texto, CSV o JSON a la consola, a un archivo o a otro programa
- Muestra y cambia por nombre el modo, el rango y los indicadores del multímetro, para usar desde scripts
- Ejecuta una captura guiada para informar de un error o verificar un multímetro nuevo

<!-- snippet via=dcma-boot-refresh.replay
dmm-cli read --interval-ms 2500 --count 5
-->
```
$ dmm-cli read --interval-ms 2500 --count 5
0.00 mA
32.54 mA
100.96 mA
107.95 mA
4.01 mA

--- 5 lecturas | Mín.: 0.00 mA | Máx.: 107.95 mA | Media: 49.09 mA | σ: 52.13 mA
```
<!-- /snippet -->

[Ayuda de dmm-cli](docs/cli-reference.md) (en inglés): la guía completa de la herramienta de línea de comandos.

## Dispositivos compatibles

| Multímetro | USB | Bluetooth | Incidencia |
|---|---|---|---|
| UNI-T UT61E+ | ✅ Verificado | ✅ Verificado (adaptador) | — |
| UNI-T UT61B+ | ✅ Verificado | 🧪 Experimental (adaptador) | [#19](https://github.com/antoinecellerier/dmm-tools/issues/19) |
| UNI-T UT161E | ✅ Verificado | 🧪 Experimental (adaptador) | [#7](https://github.com/antoinecellerier/dmm-tools/issues/7) |
| UNI-T UT61D+, UT161B/D | 🧪 Experimental | 🧪 Experimental (adaptador) | [#7](https://github.com/antoinecellerier/dmm-tools/issues/7) |
| UNI-T UT804 | ✅ Verificado | — | [#16](https://github.com/antoinecellerier/dmm-tools/issues/16) |
| UNI-T UT803 | 🧪 Experimental | — | [#15](https://github.com/antoinecellerier/dmm-tools/issues/15) |
| UNI-T UT181A | 🟡 Parcialmente verificado | 🧪 Experimental (adaptador) | [#5](https://github.com/antoinecellerier/dmm-tools/issues/5) |
| UNI-T UT171A/B/C | 🧪 Experimental | 🧪 Experimental (adaptador) | [#4](https://github.com/antoinecellerier/dmm-tools/issues/4) |
| UNI-T UT60BT | — | 🧪 Experimental (integrado) | [#26](https://github.com/antoinecellerier/dmm-tools/issues/26) |
| UNI-T UT202BT | — | 🧪 Experimental (integrado) | [#27](https://github.com/antoinecellerier/dmm-tools/issues/27) |
| UNI-T UT71A/B, UT71C/D/E | 🧪 Experimental | 🧪 Experimental (adaptador) | [#22](https://github.com/antoinecellerier/dmm-tools/issues/22) |
| UNI-T UT8802, UT8802N | 🧪 Experimental | — | [#12](https://github.com/antoinecellerier/dmm-tools/issues/12) |
| UNI-T UT8803, UT8803E | 🧪 Experimental | — | [#3](https://github.com/antoinecellerier/dmm-tools/issues/3) |
| OWON CMS061 (UNI-T UT217A) | — | ✅ Verificado (integrado) | [#45](https://github.com/antoinecellerier/dmm-tools/issues/45) |
| OWON B33 | — | 🧪 Experimental (integrado) | [#41](https://github.com/antoinecellerier/dmm-tools/issues/41) |
| OWON B35T+ | — | 🧪 Experimental (integrado) | [#42](https://github.com/antoinecellerier/dmm-tools/issues/42) |
| OWON B41T+ | — | 🧪 Experimental (integrado) | [#43](https://github.com/antoinecellerier/dmm-tools/issues/43) |
| OWON CM2100B | — | 🧪 Experimental (integrado) | [#44](https://github.com/antoinecellerier/dmm-tools/issues/44) |
| OWON CMS101 (UNI-T UT217B) | — | 🧪 Experimental (integrado) | [#45](https://github.com/antoinecellerier/dmm-tools/issues/45) |
| OWON OW18B/OW16B | — | 🧪 Experimental (integrado) | [#39](https://github.com/antoinecellerier/dmm-tools/issues/39) |
| OWON OW18E | — | 🧪 Experimental (integrado) | [#40](https://github.com/antoinecellerier/dmm-tools/issues/40) |
| OWON OW65B | — | 🧪 Experimental (integrado) | [#46](https://github.com/antoinecellerier/dmm-tools/issues/46) |
| OWON OW67B (Voltcraft VC871) | — | 🧪 Experimental (integrado) | [#47](https://github.com/antoinecellerier/dmm-tools/issues/47) |
| OWON OW69B (Voltcraft VC891) | — | 🧪 Experimental (integrado) | [#48](https://github.com/antoinecellerier/dmm-tools/issues/48) |
| ZOTEK ZT-5B (ANENG V05B) | — | ✅ Verificado (integrado) | [#31](https://github.com/antoinecellerier/dmm-tools/issues/31) |
| ZOTEK ZT-5BQ (ANENG ST207) | — | 🧪 Experimental (integrado) | [#30](https://github.com/antoinecellerier/dmm-tools/issues/30) |
| ZOTEK ZT-300AB (ANENG AN9002) | — | 🧪 Experimental (integrado) | [#28](https://github.com/antoinecellerier/dmm-tools/issues/28) |
| ZOTEK ZT-5566SE (ANENG AN999S) | — | 🧪 Experimental (integrado) | [#29](https://github.com/antoinecellerier/dmm-tools/issues/29) |
| Brymen BM525s (Minipa ET-2615A) | 🟡 Parcialmente verificado | — | [#36](https://github.com/antoinecellerier/dmm-tools/issues/36) |
| Brymen BM827s (Minipa ET-2517A) | 🟡 Parcialmente verificado | — | [#35](https://github.com/antoinecellerier/dmm-tools/issues/35) |
| Brymen BM521s | 🧪 Experimental | — | [#36](https://github.com/antoinecellerier/dmm-tools/issues/36) |
| Brymen BM829s/BM822s/BM821s | 🧪 Experimental | — | [#35](https://github.com/antoinecellerier/dmm-tools/issues/35) |
| Brymen BM869s/BM867s | 🧪 Experimental | — | [#34](https://github.com/antoinecellerier/dmm-tools/issues/34) |
| Brymen BM788BT/BM787BT | — | 🧪 Experimental (integrado) | [#33](https://github.com/antoinecellerier/dmm-tools/issues/33) |
| EEVblog 121GW | — | 🧪 Experimental (integrado) | [#32](https://github.com/antoinecellerier/dmm-tools/issues/32) |
| Voltcraft VC-880, VC650BT | 🧪 Experimental | — | [#13](https://github.com/antoinecellerier/dmm-tools/issues/13) |
| Voltcraft VC-890 | 🧪 Experimental | — | [#14](https://github.com/antoinecellerier/dmm-tools/issues/14) |
| Voltcraft VC915/VC925 PV | — | 🧪 Experimental (integrado) | [#49](https://github.com/antoinecellerier/dmm-tools/issues/49) |
| Voltcraft VC920/VC940/VC960 | 🧪 Experimental | — | [#23](https://github.com/antoinecellerier/dmm-tools/issues/23) |

✅ = modos y comandos confirmados en hardware real, salvo unas pocas comprobaciones puntuales. 🟡 = conexión y modos principales confirmados, quedan modos o comandos enteros por verificar. 🧪 = obtenido por ingeniería inversa del software del fabricante, aún sin probar en hardware real por ese enlace. — = no compatible por ese enlace.

El “adaptador” Bluetooth es el accesorio UT-D07A/B de UNI-T; “integrado” no necesita adaptador. Comenta en la incidencia de tu multímetro qué funciona y qué no.

En [dispositivos compatibles](docs/supported-devices.md) (en inglés) se detallan los cables, los adaptadores Bluetooth, qué hay que activar y qué se confirmó en cada modelo. ¿Tu multímetro no aparece? [Abre una incidencia](https://github.com/antoinecellerier/dmm-tools/issues/new) con su modelo y cómo se conecta.

## Inicio rápido

1. Descarga la [última versión](https://github.com/antoinecellerier/dmm-tools/releases/latest) para tu plataforma y descomprime el archivo. La primera vez que lo abras, [Windows](docs/setup.md#windows-protected-your-pc) y [macOS](docs/setup.md#macos-wont-open-dmm-cli-or-dmm-gui) (en inglés) muestran una advertencia de seguridad.
2. Da a la herramienta acceso al cable o activa el Bluetooth:
   - Linux: instala la regla udev y vuelve a conectar el cable.
     ```sh
     sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules
     ```
   - Windows: el cable CP2110 puede necesitar el [controlador de Silicon Labs](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers); los demás cables no necesitan nada.
   - macOS: no hay nada que instalar.
3. Activa la transmisión de datos del multímetro y ejecuta:
   ```sh
   dmm-cli read            # leer mediciones de forma continua
   dmm-gui                 # abrir la aplicación de escritorio
   ```

[Instalación y solución de problemas](docs/setup.md) (en inglés) trata los controladores, los detalles de udev, los equipos sin pantalla, la compilación desde el código fuente y las compilaciones de desarrollo.

## Documentación

Para usuarios:

- [Referencia de dmm-cli](docs/cli-reference.md) (en inglés)
- [Referencia de dmm-gui](docs/gui-reference.md) (en inglés)
- [Instalación y solución de problemas](docs/setup.md) (en inglés)
- [Dispositivos compatibles](docs/supported-devices.md) (en inglés)
- [Registro de cambios](CHANGELOG.md) (en inglés)

Para colaboradores: [CONTRIBUTING.md](CONTRIBUTING.md) para informes de errores, capturas de protocolo y cambios de código; la [guía de desarrollo](docs/development.md), la [arquitectura](docs/architecture.md), los [detalles del protocolo](docs/protocol.md), el [diseño de la experiencia de usuario](docs/ux-design.md) y [cómo agregar un dispositivo](docs/adding-devices.md) (todo en inglés).

## Comentarios

Errores, ideas, preguntas o algo de la aplicación o de la documentación que te haya confundido: [abre una incidencia](https://github.com/antoinecellerier/dmm-tools/issues/new). Si tu multímetro está en la [tabla de dispositivos](#dispositivos-compatibles), cuéntanos cómo te funciona en la incidencia de su fila. [Qué incluir en un informe de error](CONTRIBUTING.md#bug-reports) (en inglés).

## Licencia

GPL-3.0-or-later. Consulta [LICENSE](LICENSE) para ver los detalles.

## Agradecimientos

Gracias a estos proyectos de la comunidad, cuyo trabajo orientó el nuestro o sirvió para comprobarlo:

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) y [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — protocolo del UT61E+, en Python y Go
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) — capturas y analizador del UT171
- [antage/ut181a](https://github.com/antage/ut181a) y [libsigrok `uni-t-ut181a`](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) — protocolo del UT181A
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — tablas e indicadores del VC-880
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — especificación HID a UART del CP2110; [UNI-T](https://meters.uni-trend.com/download/ut61b-d-e-user-manual/) — el manual de usuario del UT61+
