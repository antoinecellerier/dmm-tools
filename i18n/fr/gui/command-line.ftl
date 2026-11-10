### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters. What each option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = Appareil auquel se connecter. Remplace pour cette session les paramètres enregistrés.
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = Fixer l’appareil simulé sur un mode de mesure précis au lieu de parcourir les modes automatiquement. Implique --device mock et remplace pour cette session la valeur enregistrée du paramètre Mode simulé.
