<!-- translated from README.md 5de2854e974370ecfc417d2bd201cb7398fd7b07 -->
# dmm-tools

[![CI](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/antoinecellerier/dmm-tools/actions/workflows/ci.yml)
[![Version](https://img.shields.io/github/v/release/antoinecellerier/dmm-tools)](https://github.com/antoinecellerier/dmm-tools/releases/latest)
[![Licence : GPL-3.0-or-later](https://img.shields.io/github/license/antoinecellerier/dmm-tools)](LICENSE)

[English](README.md) · [Español](README.es.md) · **Français** · [Português (Brasil)](README.pt-BR.md) · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

*Cette page est une traduction du [README en anglais](README.md), qui fait foi.*

Application de bureau et outil en ligne de commande open source pour lire, enregistrer et tracer les mesures des multimètres numériques (DMM) et des pinces multimètres des marques UNI-T, ZOTEK (ZOYI, ANENG, BSIDE), OWON, Brymen (Minipa), EEVblog et Voltcraft, et les commander à distance, en USB et en Bluetooth, sous Linux, Windows et macOS, et sur Raspberry Pi.

Fonctionne avec les multimètres UNI-T UT61E+, UT181A, UT161, UT71, UT60BT et UT804, ANENG AN9002 (ZOTEK ZT-300AB), Brymen BM869s, EEVblog 121GW, OWON OW18E et B41T+, Voltcraft VC-890 et [d’autres](#appareils-pris-en-charge).

**Sommaire :** [Fonctionnalités](#fonctionnalités) · [Appareils pris en charge](#appareils-pris-en-charge) · [Démarrage rapide](#démarrage-rapide) · [Documentation](#documentation) · [Commentaires](#commentaires) · [Licence](#licence)

![dmm-gui pendant une mesure de courant continu en mA : la mesure en direct, les spécifications et les statistiques à côté du graphique, où la minimap fait ressortir la séquence de démarrage d’un capteur, deux curseurs en lisent la durée et la charge, et des marqueurs en nomment les étapes](assets/gui-wide-layout.fr.webp)

## Fonctionnalités

L’application de bureau et l’outil en ligne de commande :

- Identifient le multimètre connecté
- Commandent le multimètre à distance : changent son mode et sa plage, appuient sur ses touches
- Enregistrent les mesures en CSV, en JSON ou dans un fichier de relecture, et les rouvrent
- Calculent des statistiques : min, max, moyenne, écart-type et intégrale temporelle
- Déclenchent une alarme quand une mesure franchit une limite
- Appliquent aux mesures, par logiciel, une mise à l’échelle, un décalage et une nouvelle étiquette d’unité
- Affichent leur interface en anglais, chinois simplifié, espagnol, français, japonais ou portugais du Brésil, selon la langue du système

### Application de bureau (`dmm-gui`)

De plus, l’application de bureau :

- Affiche la mesure en direct avec la résolution et la précision de la plage, tirées du manuel
- Trace les mesures en direct, avec des curseurs, des lignes de référence, une enveloppe min/max et un histogramme
- Enregistre de longues sessions, avec des marqueurs et des notes sur n’importe quelle mesure
- Remplit la fenêtre avec la mesure en mode grand affichage

[Aide de l’interface graphique](docs/gui-reference.md) (en anglais) : le guide complet de l’application de bureau.

### Outil en ligne de commande (`dmm-cli`)

De plus, l’outil en ligne de commande :

- Transmet en continu les mesures en texte, CSV ou JSON vers le terminal, un fichier ou un autre programme
- Liste et modifie par leur nom le mode, la plage et les indicateurs du multimètre, pour les scripts
- Lance une capture guidée pour signaler un bug ou vérifier un nouveau multimètre

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

--- 5 mesures | Min : 0.00 mA | Max : 107.95 mA | Moyenne : 49.09 mA | σ : 52.13 mA
```
<!-- /snippet -->

[Aide de la ligne de commande](docs/cli-reference.md) (en anglais) : le guide complet de l’outil en ligne de commande.

## Appareils pris en charge

| Multimètre | USB | Bluetooth | Ticket |
|---|---|---|---|
| UNI-T UT61E+ | ✅ Vérifié | ✅ Vérifié (adaptateur) | — |
| UNI-T UT61B+ | ✅ Vérifié | 🧪 Expérimental (adaptateur) | [#19](https://github.com/antoinecellerier/dmm-tools/issues/19) |
| UNI-T UT161E | ✅ Vérifié | 🧪 Expérimental (adaptateur) | [#7](https://github.com/antoinecellerier/dmm-tools/issues/7) |
| UNI-T UT61D+, UT161B/D | 🧪 Expérimental | 🧪 Expérimental (adaptateur) | [#7](https://github.com/antoinecellerier/dmm-tools/issues/7) |
| UNI-T UT804 | ✅ Vérifié | — | [#16](https://github.com/antoinecellerier/dmm-tools/issues/16) |
| UNI-T UT803 | 🧪 Expérimental | — | [#15](https://github.com/antoinecellerier/dmm-tools/issues/15) |
| UNI-T UT181A | 🟡 Partiellement vérifié | 🧪 Expérimental (adaptateur) | [#5](https://github.com/antoinecellerier/dmm-tools/issues/5) |
| UNI-T UT171A/B/C | 🧪 Expérimental | 🧪 Expérimental (adaptateur) | [#4](https://github.com/antoinecellerier/dmm-tools/issues/4) |
| UNI-T UT60BT | — | 🧪 Expérimental (intégré) | [#26](https://github.com/antoinecellerier/dmm-tools/issues/26) |
| UNI-T UT202BT | — | 🧪 Expérimental (intégré) | [#27](https://github.com/antoinecellerier/dmm-tools/issues/27) |
| UNI-T UT71A/B, UT71C/D/E | 🧪 Expérimental | 🧪 Expérimental (adaptateur) | [#22](https://github.com/antoinecellerier/dmm-tools/issues/22) |
| UNI-T UT8802, UT8802N | 🧪 Expérimental | — | [#12](https://github.com/antoinecellerier/dmm-tools/issues/12) |
| UNI-T UT8803, UT8803E | 🧪 Expérimental | — | [#3](https://github.com/antoinecellerier/dmm-tools/issues/3) |
| ZOTEK ZT-5B (ANENG V05B) | — | ✅ Vérifié (intégré) | [#31](https://github.com/antoinecellerier/dmm-tools/issues/31) |
| ZOTEK ZT-5BQ (ANENG ST207) | — | 🧪 Expérimental (intégré) | [#30](https://github.com/antoinecellerier/dmm-tools/issues/30) |
| ZOTEK ZT-300AB (ANENG AN9002) | — | 🧪 Expérimental (intégré) | [#28](https://github.com/antoinecellerier/dmm-tools/issues/28) |
| ZOTEK ZT-5566SE (ANENG AN999S) | — | 🧪 Expérimental (intégré) | [#29](https://github.com/antoinecellerier/dmm-tools/issues/29) |
| Brymen BM525s (Minipa ET-2615A) | 🟡 Partiellement vérifié | — | [#36](https://github.com/antoinecellerier/dmm-tools/issues/36) |
| Brymen BM827s (Minipa ET-2517A) | 🟡 Partiellement vérifié | — | [#35](https://github.com/antoinecellerier/dmm-tools/issues/35) |
| Brymen BM521s | 🧪 Expérimental | — | [#36](https://github.com/antoinecellerier/dmm-tools/issues/36) |
| Brymen BM829s/BM822s/BM821s | 🧪 Expérimental | — | [#35](https://github.com/antoinecellerier/dmm-tools/issues/35) |
| Brymen BM869s/BM867s | 🧪 Expérimental | — | [#34](https://github.com/antoinecellerier/dmm-tools/issues/34) |
| Brymen BM788BT/BM787BT | — | 🧪 Expérimental (intégré) | [#33](https://github.com/antoinecellerier/dmm-tools/issues/33) |
| EEVblog 121GW | — | 🧪 Expérimental (intégré) | [#32](https://github.com/antoinecellerier/dmm-tools/issues/32) |
| OWON B33 | — | 🧪 Expérimental (intégré) | [#41](https://github.com/antoinecellerier/dmm-tools/issues/41) |
| OWON B35T+ | — | 🧪 Expérimental (intégré) | [#42](https://github.com/antoinecellerier/dmm-tools/issues/42) |
| OWON B41T+ | — | 🧪 Expérimental (intégré) | [#43](https://github.com/antoinecellerier/dmm-tools/issues/43) |
| OWON CM2100B | — | 🧪 Expérimental (intégré) | [#44](https://github.com/antoinecellerier/dmm-tools/issues/44) |
| OWON CMS061 (UNI-T UT217A) | — | 🟡 Partiellement vérifié (intégré) | [#45](https://github.com/antoinecellerier/dmm-tools/issues/45) |
| OWON CMS101 (UNI-T UT217B) | — | 🧪 Expérimental (intégré) | [#45](https://github.com/antoinecellerier/dmm-tools/issues/45) |
| OWON OW18B/OW16B | — | 🧪 Expérimental (intégré) | [#39](https://github.com/antoinecellerier/dmm-tools/issues/39) |
| OWON OW18E | — | 🧪 Expérimental (intégré) | [#40](https://github.com/antoinecellerier/dmm-tools/issues/40) |
| OWON OW65B | — | 🧪 Expérimental (intégré) | [#46](https://github.com/antoinecellerier/dmm-tools/issues/46) |
| OWON OW67B (Voltcraft VC871) | — | 🧪 Expérimental (intégré) | [#47](https://github.com/antoinecellerier/dmm-tools/issues/47) |
| OWON OW69B (Voltcraft VC891) | — | 🧪 Expérimental (intégré) | [#48](https://github.com/antoinecellerier/dmm-tools/issues/48) |
| Voltcraft VC-880, VC650BT | 🧪 Expérimental | — | [#13](https://github.com/antoinecellerier/dmm-tools/issues/13) |
| Voltcraft VC-890 | 🧪 Expérimental | — | [#14](https://github.com/antoinecellerier/dmm-tools/issues/14) |
| Voltcraft VC915/VC925 PV | — | 🧪 Expérimental (intégré) | [#49](https://github.com/antoinecellerier/dmm-tools/issues/49) |
| Voltcraft VC920/VC940/VC960 | 🧪 Expérimental | — | [#23](https://github.com/antoinecellerier/dmm-tools/issues/23) |

✅ = modes et commandes confirmés sur du matériel réel, à quelques vérifications ponctuelles près. 🟡 = connexion et modes principaux confirmés, certains modes ou commandes restent entièrement à vérifier. 🧪 = obtenu par rétro-ingénierie du logiciel du fabricant, pas encore testé sur du matériel réel avec cette liaison. — = non pris en charge sur cette liaison.

En Bluetooth, « adaptateur » désigne le module complémentaire UT-D07A/B d’UNI-T. « Intégré » : aucun adaptateur n’est nécessaire. Indiquez dans le ticket de votre multimètre ce qui fonctionne et ce qui ne fonctionne pas.

La page [Appareils pris en charge](docs/supported-devices.md) (en anglais) détaille les câbles, les adaptateurs Bluetooth, ce que vous devez activer et ce qui a été confirmé pour chaque modèle. Votre multimètre n’est pas dans la liste ? [Ouvrez un ticket](https://github.com/antoinecellerier/dmm-tools/issues/new) en indiquant son modèle et son mode de connexion.

## Démarrage rapide

1. Téléchargez la [dernière version](https://github.com/antoinecellerier/dmm-tools/releases/latest) pour votre plateforme et décompressez l’archive. Au premier lancement, un avertissement de sécurité s’affiche sous [Windows](docs/setup.md#windows-protected-your-pc) et [macOS](docs/setup.md#macos-wont-open-dmm-cli-or-dmm-gui) (en anglais).
2. Donnez à l’outil l’accès au câble, ou activez le Bluetooth :
   - Linux : installez la règle udev, puis rebranchez le câble.
     ```sh
     sudo cp udev/70-dmm-tools.rules /etc/udev/rules.d/ && sudo udevadm control --reload-rules
     ```
   - Windows : le câble CP2110 peut demander le [pilote Silicon Labs](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers). Les autres câbles fonctionnent sans pilote.
   - macOS : rien à installer.
3. Activez la transmission des données du multimètre et lancez :
   ```sh
   dmm-cli read            # lire les mesures en continu
   dmm-gui                 # lancer l’interface graphique
   ```

Le guide [Installation et dépannage](docs/setup.md) (en anglais) couvre les pilotes, les détails d’udev, les machines sans écran, la compilation depuis les sources et les versions de développement.

## Documentation

Pour les utilisateurs :

- [Référence de la ligne de commande](docs/cli-reference.md) (en anglais)
- [Référence de l’interface graphique](docs/gui-reference.md) (en anglais)
- [Installation et dépannage](docs/setup.md) (en anglais)
- [Appareils pris en charge](docs/supported-devices.md) (en anglais)
- [Journal des modifications](CHANGELOG.md) (en anglais)

Pour les contributeurs, en anglais : [CONTRIBUTING.md](CONTRIBUTING.md) pour les rapports de bug, les captures de protocole et les modifications du code, puis le [guide de développement](docs/development.md), l’[architecture](docs/architecture.md), les [détails des protocoles](docs/protocol.md), la [conception de l’interface](docs/ux-design.md) et l’[ajout d’un appareil](docs/adding-devices.md).

## Commentaires

Bugs, idées, questions, ou un point de l’application ou de la documentation qui vous a semblé peu clair : [ouvrez un ticket](https://github.com/antoinecellerier/dmm-tools/issues/new). Pour un multimètre du [tableau des appareils](#appareils-pris-en-charge), donnez vos retours dans le ticket indiqué sur sa ligne. [Que mettre dans un rapport de bug](CONTRIBUTING.md#bug-reports) (en anglais).

## Licence

GPL-3.0-or-later. Consultez [LICENSE](LICENSE) (en anglais) pour plus de détails.

## Remerciements

Merci aux projets communautaires dont le travail a guidé le nôtre ou a permis de le recouper :

- [ljakob/unit_ut61eplus](https://github.com/ljakob/unit_ut61eplus) et [mwuertinger/ut61ep](https://github.com/mwuertinger/ut61ep) — protocole de l’UT61E+, en Python et en Go
- [gulux/Uni-T-CP2110](https://github.com/gulux/Uni-T-CP2110) — captures et analyseur de l’UT171
- [antage/ut181a](https://github.com/antage/ut181a) et [libsigrok `uni-t-ut181a`](https://github.com/sigrokproject/libsigrok/tree/master/src/hardware/uni-t-ut181a) — protocole de l’UT181A
- [pylablib](https://github.com/AlexShkarin/pyLabLib) — tables et indicateurs du VC-880
- [Silicon Labs AN434](https://www.silabs.com/documents/public/application-notes/an434-cp2110-4-interface-specification.pdf) — spécification de l’interface HID vers UART du CP2110 ; [UNI-T](https://meters.uni-trend.com/download/ut61b-d-e-user-manual/) — le manuel d’utilisation des UT61+
