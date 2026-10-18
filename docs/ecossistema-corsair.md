# Projetos abertos da Corsair: o que aproveitar

Levantamento de 2026-10-17. Código lido só para entender protocolos e organização;
nada foi copiado (todos são GPL, ver "Licença" no fim).

## Projetos

| Projeto | Licença | Ativo | Cobre | Útil para nós |
|---|---|---|---|---|
| [OpenLinkHub](https://github.com/jurkovic-nikola/OpenLinkHub) | GPL-3.0 | sim (out/2026) | ~180 PIDs: teclados, mouses, fones, hubs, AIOs | **o mais completo para o HS80 e o K70 MK.2** |
| [OpenRGB](https://gitlab.com/CalcProgrammer1/OpenRGB) | GPL-2.0 | sim | ~100 PIDs Corsair, só iluminação | layouts de teclado, detecção por interface/usage |
| [ckb-next](https://github.com/ckb-next/ckb-next) | GPL-2.0 | sim | ~90 PIDs, teclados e mouses (protocolo "NXP") | significado dos modos de tecla, layouts por idioma |
| [liquidctl](https://github.com/liquidctl/liquidctl) | GPL-3.0 | sim | 40 PIDs: AIOs, fontes, Commander | **regras udev geradas da tabela de dispositivos** |
| [HeadsetControl](https://github.com/Sapd/HeadsetControl) | GPL-3.0 | sim | 7 PIDs Corsair (Void, Virtuoso) | não suporta o HS80 |
| [hs80tray](https://github.com/robertoszek/hs80tray) | MIT | pouco | HS80: bateria | troca a saída de som ao ligar o fone |
| kernel `hid-corsair-void`, `corsair-psu`, `corsair-cpro` | GPL-2.0 | sim | Void, fontes, Commander Pro | drivers que o Linux já traz; não brigar com eles |

Somando as tabelas: **229 PIDs diferentes**, e 104 aparecem em um projeto só. Nenhum
tem a lista completa, e cada um guarda a sua no código (mapa Go, macros C++, tuplas
Python). O `0A6A` do HS80 (cabo com o fone desligado) não está em nenhum.

## Melhorias para os nossos aparelhos

Fonte entre colchetes. Todas precisam ser confirmadas no hardware antes de virar ✅.

### K70 RGB MK.2 Low Profile
1. **Modo performance no firmware** [OpenLinkHub]: `07 48 xx`, com `xx = 0xA0` + 1
   (bloqueia Windows) + 2 (Alt+Tab) + 4 (Alt+F4) + 8 (Shift+Tab). O Win Lock passaria a
   valer mesmo com o app fechado, e daria para oferecer os outros três bloqueios
   para jogos.
2. **Taxa de polling** [OpenLinkHub]: `07 0A 00 00 ms`, com ms = 1/2/4/8
   (1000/500/250/125 Hz).
3. Perfil salvo no teclado (memória interna): os três projetos escrevem, mas o risco
   de corromper a memória continua; fica para depois.

### HS80 RGB Wireless
4. **Desligamento automático** [OpenLinkHub]: SET `0D` = 1/0 liga ou desliga; SET `0E` =
   tempo em ms (u32).
5. **Mute do microfone** [OpenLinkHub]: GET `A6` (1 = mudo) e notificações `8E`/`A6`.
   Permite acender o LED do microfone (hoje apagado no modo software) e sincronizar o
   mute do sistema.
6. **Versão do firmware**: GET `13`.
7. **Bateria por notificação** (`03 01 01 0F …`) em vez de consultar a cada 5 s.
8. Ao ligar o fone, tornar o HS80 a saída de som padrão (opcional) [hs80tray].
9. Sidetone: nenhum projeto tem para o HS80; precisa de captura do iCUE.

## Banco central "plug and play"

Hoje `registry.rs` tem 3 aparelhos fixos no código. A proposta é separar **dados**
(qual aparelho é o quê) de **drivers** (como falar com cada família de protocolo):

```toml
# devices/corsair.toml (embutido no binário; um arquivo do usuário pode completar)
[[device]]
pid      = 0x1B55
name     = "K70 RGB MK.2 Low Profile"
kind     = "keyboard"
family   = "nxp"            # driver: 07/0E/7F de 64 bytes
control  = { command = "FFC2:0004", reply = "FFC2:0003", keys = "FFC0:0002" }
leds     = 168
layout   = "k70_mk2_lp"     # geometria + mapa de LEDs por idioma
features = ["rgb-per-key", "key-modes", "performance-mode", "polling-rate"]
status   = "verified"       # verified | community | untested

[[device]]
pid      = 0x0A6B
name     = "HS80 RGB Wireless (receptor)"
kind     = "headset-dongle"
family   = "bragi"
control  = { command = "FF42:0001", events = "FF42:0002" }
pairs    = [0x0A69]         # o mesmo fone no cabo
features = ["battery", "leds-3", "wheel-click", "sleep-timer", "mic-mute"]
status   = "verified"
```

- **Famílias = drivers**: `nxp` (K70/K95/Strafe MK.2, o que temos), `bragi` (HS80, K70
  PRO/CORE, mouses e dongles novos), depois `link`/`commander` se fizer sentido. Um
  aparelho novo da mesma família funciona só com uma linha no banco.
- **`status`** controla o que o app faz: `verified` liga tudo; `community` liga e avisa;
  `untested` só lê (bateria, firmware) até alguém confirmar.
- **`features`** decide o que a interface mostra, em vez de código fixo por aparelho.
- **Regras udev geradas do banco** (como o liquidctl), e o `.rpm` instala o resultado.
- **Semente**: os 229 PIDs com nome e família podem entrar como `untested`, com a
  fonte de cada um anotada; só viram `verified` com teste no hardware.

## Licença

Todos os projetos que trazem código útil são GPL. Fatos (IDs USB, bytes de comando)
podem ser usados com crédito em qualquer licença; **copiar ou portar código** deles
exige que o OpenPeripheral seja GPL também. A escolha da licença decide se dá para
aproveitar código ou só a documentação.

**Decisão (2026-10-17): GPL-3.0.** Portar código desses projetos é permitido, mantendo o
crédito ao projeto de origem no arquivo e no commit.

| Projeto | Termos | Portar código para GPL-3.0? |
|---|---|---|
| OpenLinkHub, liquidctl, HeadsetControl | GPL-3.0 | sim |
| OpenRGB | `GPL-2.0-or-later` (SPDX nos arquivos) | sim |
| ckb-next | "GPL version 2", sem "or later" e sem SPDX nos arquivos | **não**: só fatos |
