### The parts of dmm-gui's --help built as it runs: the first lines of the
### help of --device and --mock-mode, which go on with lists made from the
### meters. What each option does is in gui-help.ftl.
###
### Words in --flag form are typed by the user and stay as they are.

## The first lines of two options' long help (dmm-gui --help).

# Before the list of devices.
command-line-device-intro = Dispositivo ao qual se conectar. Nesta sessão, tem prioridade sobre as configurações salvas.
# Before the list of the simulated meter's modes. "Mock mode" is the
# setting's name in the GUI's Settings.
command-line-mock-mode-intro = Fixar o dispositivo simulado em um modo de medição específico, em vez de percorrer todos os modos automaticamente. Ativa também --device mock e, nesta sessão, tem prioridade sobre o valor salvo da configuração Modo simulado.
