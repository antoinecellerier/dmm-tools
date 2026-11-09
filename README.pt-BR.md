<!-- translated from README.md b3f3a0a3722dd935b8bf521ad26efad3a0c63a18 -->
# dmm-tools

[![CI](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml)
[![Versão](https://img.shields.io/github/v/release/antoinecellerier/dmm-tools)](https://github.com/antoinecellerier/dmm-tools/releases/latest)
[![Licença: GPL-3.0-or-later](https://img.shields.io/github/license/antoinecellerier/dmm-tools)](LICENSE)

[English](README.md) · [Español](README.es.md) · [Français](README.fr.md) · **Português (Brasil)** · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

*Esta página foi traduzida automaticamente; a referência é o [README em inglês](README.md). Uma revisão por falante nativo é bem-vinda: [como ajudar](CONTRIBUTING.md#translations) (em inglês).*

Aplicativo de desktop e ferramenta de linha de comando, ambos de código aberto, para ler, gravar, plotar e controlar remotamente multímetros digitais (DMMs) e alicates amperímetros das marcas UNI-T, ZOTEK (ZOYI, ANENG, BSIDE), OWON, Brymen (Minipa), EEVblog e Voltcraft, via USB e Bluetooth, no Linux, Windows, macOS e Raspberry Pi.

Funciona com UNI-T UT61E+, UT181A, UT161, UT71, UT60BT e UT804, ANENG AN9002 (ZOTEK ZT-300AB), Brymen BM869s, EEVblog 121GW, OWON OW18E e B41T+, Voltcraft VC-890 e [outros](#dispositivos-compatíveis).

**Sumário:** [Recursos](#recursos) · [Dispositivos compatíveis](#dispositivos-compatíveis) · [Início rápido](#início-rápido) · [Documentação](#documentação) · [Sugestões](#sugestões) · [Licença](#licença)

![dmm-gui em uma sessão de DC mA: a leitura ao vivo, as especificações e as estatísticas ao lado do gráfico, onde o minimapa destaca a sequência de inicialização de um sensor, dois cursores medem a duração e a carga dessa sequência e marcadores nomeiam as etapas](assets/gui-wide-layout.pt-BR.webp)

## Recursos

Tanto o aplicativo de desktop quanto a ferramenta de linha de comando:

- Detectam qual multímetro está conectado
- Controlam o multímetro remotamente: mudam o modo e a faixa e pressionam as teclas
- Salvam as leituras em CSV, JSON ou arquivo de reprodução e as abrem de novo
- Calculam estatísticas: mínimo, máximo, média, desvio padrão e a integral ao longo do tempo
- Disparam um alarme quando uma leitura ultrapassa um limite
- Aplicam escala, deslocamento e um novo rótulo de unidade às leituras, por software
- Mostram o texto em inglês, chinês simplificado, espanhol, francês, japonês ou português do Brasil, seguindo o idioma do sistema

### Aplicativo de desktop (`dmm-gui`)

O aplicativo de desktop também:

- Mostra a leitura ao vivo com a resolução e a precisão da faixa segundo o manual
- Plota as leituras ao vivo, com cursores, linhas de referência, um envelope de mínimo/máximo e um histograma
- Grava sessões longas, com marcadores e notas em qualquer leitura
- No modo de leitura ampliada, mostra a leitura ocupando a janela inteira

[Ajuda da GUI](docs/gui-reference.md) (em inglês): o guia completo do aplicativo de desktop.

### Ferramenta de linha de comando (`dmm-cli`)

A ferramenta de linha de comando também:

- Transmite as leituras em texto, CSV ou JSON para o terminal, um arquivo ou outro programa
- Lista e altera o modo, a faixa e os indicadores do multímetro por nome, para uso em scripts
- Executa uma captura guiada para relatar um bug ou verificar um novo multímetro

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

--- 5 leituras | Mín.: 0.00 mA | Máx.: 107.95 mA | Média: 49.09 mA | σ: 52.13 mA
```
<!-- /snippet -->

[Ajuda da CLI](docs/cli-reference.md) (em inglês): o guia completo da ferramenta de linha de comando.

## Dispositivos compatíveis

| Multímetro | USB | Bluetooth | Issue |
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

✅ = modos e comandos confirmados em hardware real, faltando apenas algumas verificações pontuais. 🟡 = conexão e modos principais confirmados; modos inteiros ou comandos ainda a verificar. 🧪 = suporte criado por engenharia reversa do software do fabricante, ainda não testado em hardware real nessa conexão. — = sem suporte nessa conexão.

No Bluetooth, “adaptador” é o acessório UT-D07A/B da UNI-T; “integrado” dispensa adaptador. Na issue do seu multímetro, conte o que funciona e o que não funciona.

Veja em [dispositivos compatíveis](docs/supported-devices.md) (em inglês) os cabos, os adaptadores Bluetooth, o que ativar e o que já foi confirmado em cada modelo. Seu multímetro não está na lista? [Abra uma issue](https://github.com/antoinecellerier/dmm-tools/issues/new) com o modelo e a forma de conexão.

## Início rápido

1. Baixe a [versão mais recente](https://github.com/antoinecellerier/dmm-tools/releases/latest) para a sua plataforma e extraia o arquivo. Na primeira execução, o [Windows](docs/setup.md#windows-protected-your-pc) e o [macOS](docs/setup.md#macos-wont-open-dmm-cli-or-dmm-gui) mostram um aviso de segurança (instruções em inglês).
2. Dê à ferramenta acesso ao cabo ou ligue o Bluetooth:
   - Linux: instale a regra do udev e reconecte o cabo.
     ```sh
     sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules
     ```
   - Windows: o cabo CP2110 pode precisar do [driver da Silicon Labs](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers); os outros cabos não precisam de nada.
   - macOS: não é preciso instalar nada.
3. Ligue a transmissão de dados do multímetro e execute:
   ```sh
   dmm-cli read            # transmitir as leituras
   dmm-gui                 # abrir a GUI
   ```

[Configuração e solução de problemas](docs/setup.md) (em inglês) trata de drivers, detalhes do udev, máquinas sem interface gráfica, compilação a partir do código-fonte e versões de desenvolvimento.

## Documentação

Para usuários:

- [Referência da CLI](docs/cli-reference.md) (em inglês)
- [Referência da GUI](docs/gui-reference.md) (em inglês)
- [Configuração e solução de problemas](docs/setup.md) (em inglês)
- [Dispositivos compatíveis](docs/supported-devices.md) (em inglês)
- [Histórico de alterações](CHANGELOG.md) (em inglês)

Para contribuidores: [CONTRIBUTING.md](CONTRIBUTING.md), sobre relatórios de bugs, capturas de protocolo e alterações de código; o [guia de desenvolvimento](docs/development.md), a [arquitetura](docs/architecture.md), os [detalhes do protocolo](docs/protocol.md), o [design de UX](docs/ux-design.md) e [como adicionar um dispositivo](docs/adding-devices.md) (todos em inglês).

## Sugestões

Para relatar bugs, sugerir ideias, tirar dúvidas ou apontar algo confuso no aplicativo ou na documentação, [abra uma issue](https://github.com/antoinecellerier/dmm-tools/issues/new). Para contar sua experiência com um multímetro da [tabela de dispositivos](#dispositivos-compatíveis), use a issue da linha correspondente. [O que incluir em um relatório de bug](CONTRIBUTING.md#bug-reports) (em inglês).

## Licença

GPL-3.0-or-later. Veja [LICENSE](LICENSE) para mais detalhes.

## Agradecimentos

Agradecemos aos projetos da comunidade cujo trabalho serviu de base para o nosso ou ajudou a verificá-lo:

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) e [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — protocolo do UT61E+, em Python e Go
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) — capturas e analisador do UT171
- [antage/ut181a](https://github.com/antage/ut181a) e [libsigrok `uni-t-ut181a`](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) — protocolo do UT181A
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — tabelas e indicadores do VC-880
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — especificação HID-to-UART do CP2110; [UNI-T](https://meters.uni-trend.com/download/ut61b-d-e-user-manual/) — o manual de instruções do UT61+
