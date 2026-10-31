# iCUE 5 × OpenPeripheral: o que falta

Levantamento de 2026-10-31 para os três aparelhos do projeto: K70 RGB MK.2 Low Profile,
HS80 RGB Wireless e M65 RGB Elite. As fontes são a ajuda e os artigos da Corsair, as notas
de versão do iCUE, os manuais e as análises (links abaixo). Não foi feito no iCUE instalado.
O que ficou sem confirmação está marcado **(confirmar)**.

Legenda: **tem** · **parcial** · **falta** · valor para quem usa (alto/médio/baixo).

## K70 RGB MK.2 Low Profile

| Função no iCUE | Nós | Valor | Como fazer |
|---|---|---|---|
| Iluminação por tecla e efeitos | parcial: temos fixo, arco-íris, onda, respiração e reativo; faltam Chuva, Visor, Color Shift, Spiral Rainbow, Type Ripple e Gradiente | médio | só software |
| Camadas de iluminação (vários efeitos empilhados, em todas ou em algumas teclas) | falta (temos um efeito mais a pintura) | médio-alto | só software |
| Perfis gravados no teclado (3 posições, funcionam sem o app e em outro PC) | falta | alto | protocolo `PROFILE.I` em `protocol/k70_mk2_lp.md`; o ckb-next serve de referência |
| Win Lock com Alt+Tab/Alt+F4/Shift+Tab, polling, teclas de brilho e de perfil | tem | — | — |
| Remapear, macros, texto, mídia, abrir app, trocar perfil, desativar | tem | — | — |
| Ação "Timer" (contagem, depois som, luz ou macro) | falta | baixo-médio | só software |
| Efeito que reage à temperatura da CPU/GPU | falta | baixo-médio | hwmon no Linux; LibreHardwareMonitor no Windows |
| Atualizar firmware | falta | baixo (o MK.2 está fora de linha) | captura + arquivos da Corsair; risco de travar o teclado |

## HS80 RGB Wireless

| Função no iCUE | Nós | Valor | Como fazer |
|---|---|---|---|
| **Sidetone** (ouvir a própria voz no fone, com nível) | falta | **alto** | no Linux pode ser só um controle do ALSA (`amixer -c HS80 controls`) (confirmar); no Windows precisa de captura |
| **Volume do microfone** | falta (só mostramos se está mudo) | **alto** | PipeWire/ALSA no Linux; volume do dispositivo no Windows |
| **Mic Boost** (+10 dB; o microfone do HS80 sem fio é baixo) | falta | **alto** | ganho por software: filtro do PipeWire; pré-amplificação no Equalizer APO |
| Redução de ruído do microfone (NVIDIA Broadcast dentro do iCUE) | falta | médio-alto | RNNoise ou DeepFilterNet como filtro do PipeWire; no Windows é mais difícil |
| Luz das conchas (2 zonas, efeitos, brilho, desligar para poupar bateria: cerca de 15 h acesa e mais de 20 h apagada) | parcial: o LED só mostra a cor da bateria | médio | captura do comando de luz (ver se o OpenRGB já tem) |
| Equalizador de 10 bandas, clique na roda troca o preset, desligar sozinho | tem | — | dá para salvar presets próprios |
| Bateria | tem (o iCUE só mostra alta/média/baixa) | — | dá para superar: aviso de bateria baixa e histórico |
| Som espacial (Dolby Atmos / Windows Sonic) | falta | médio | o Dolby é licenciado e só existe no Windows; no Linux, 7.1 virtual (HRTF) num filtro do PipeWire |
| Parear fone e receptor, atualizar firmware | falta | baixo-médio | captura; risco |

## M65 RGB Elite

| Função no iCUE | Nós | Valor | Como fazer |
|---|---|---|---|
| 5 estágios de DPI + sniper, cor por estágio, polling, angle snapping, resposta dos botões, 8 botões e macros | tem | — | — |
| Luz por zona (logo, roda) | tem | — | faltam alguns dos 13 efeitos do iCUE |
| Calibrar no mousepad / distância de levantamento | falta | médio | captura; não se sabe se o iCUE 5 ainda oferece (confirmar) |
| Perfil gravado no mouse (1 posição) | falta | alto | as fontes não concordam se dá para gravar (confirmar) |
| Atalho para velocidade do ponteiro do sistema | falta | baixo | só configuração do sistema operacional |

## O app

| Função no iCUE | Nós | Valor | Como fazer |
|---|---|---|---|
| Perfis que trocam sozinhos pelo programa na frente | tem (Windows; Linux com X11/XWayland) | — | falta Wayland nativo (KWin/GNOME) |
| **Exportar/importar perfil** em arquivo, escolhendo as partes | falta | médio | só software, barato |
| **Murals**: luz a partir de imagem, **visualizador de áudio** ou **espelho da tela**, em todos os aparelhos | falta (temos cenas) | médio-alto | captura de tela e do som pelo PipeWire (Linux) ou DXGI/WASAPI (Windows) |
| Lighting Link (o mesmo efeito em todos os aparelhos) | parcial (o mouse segue o teclado; o fone não) | médio | depende da luz do HS80 |
| Biblioteca de ações reutilizáveis | parcial | baixo-médio | só software |
| Integração com jogos (luz reage ao jogo) | falta | baixo-médio | o SDK da Corsair só existe no Windows; alternativa: servidor compatível com o OpenRGB (Artemis, Aurora) |
| Painel de sensores (CPU/GPU) | falta | baixo | serve mais ao efeito de temperatura |
| Philips Hue, Nanoleaf, Govee, Stream Deck, widgets do Xbox Game Bar | falta | baixo | fora do foco: são outros aparelhos ou só Windows |
| Ventoinhas, AIO, telas (Xeneon Edge) | não se aplica | — | — |

O "Sonic" e o Nahimic não são do iCUE (são da ASUS e da MSI/SteelSeries).

## As 10 melhores adições

1. **Sidetone** do HS80, com liga/desliga e nível.
2. **Volume do microfone e Mic Boost** do HS80.
3. **Gravar perfis no teclado** (e no mouse, se der), para valerem sem o app.
4. **Redução de ruído do microfone**.
5. **Mais efeitos e camadas de iluminação** no teclado.
6. **Luz das conchas do HS80**, com modo apagado para poupar bateria, e o mesmo efeito em todos os aparelhos.
7. **Visualizador de áudio e luz que acompanha a tela** ("Murals").
8. **Exportar/importar perfis**.
9. **Aviso de bateria baixa** e histórico da bateria do HS80.
10. **Som 7.1 virtual** no Linux.

## Fontes principais

- Ajuda da Corsair: perfis gravados no aparelho (help.corsair.com/hc/en-us/articles/360043423432),
  perfis com programa ligado (…/360058672092), exportar perfis (…/9399098258189),
  Mic Boost (…/5827408701325), cenas (…/360058833551), jogos com integração (…/4403113239565),
  Stream Deck (…/360044964132), Govee (…/26211782625549).
- Corsair Explorer: equalizador no iCUE, macros e remapeamento, Murals
  (corsair.com/explorer/gamer/accessories/inside-the-new-murals/), série HS80, NVIDIA Broadcast.
- Notas de versão do iCUE 5.8, 5.36.80, 5.47.101 e 5.49.34.
- Análises: TechPowerUp (HS80 RGB Wireless, HS80 MAX, M65 RGB Elite, K70 MK.2 SE), Hexus,
  Notebookcheck, NikkTech, TweakTown, FunkyKit, Tech4Gamers, Tech Advisor.
- HeadsetControl (fork HixtyGamer): sidetone do HS80 como controle do ALSA.
