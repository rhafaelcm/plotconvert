# plotconvert

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Documentação em português. Para inglês, veja [README.md](README.md).

Conversor entre DXF ASCII, HP-GL/HP-GL2 (`.plt`, `.hpgl`) e SVG (`.svg`, `.svf`),
escrito em Rust.

O conversor suporta seis rotas de conversão cruzada entre formatos vetoriais,
além de **PNG** como saída raster a partir de qualquer entrada:

- **DXF** → PLT, SVG, PNG
- **PLT/HP-GL** → DXF, SVG, PNG
- **SVG/SVF** → DXF, PLT, PNG

| Entrada | Saídas |
| --- | --- |
| DXF (`.dxf`) | PLT, SVG ou PNG |
| PLT/HP-GL (`.plt`, `.hpgl`) | DXF, SVG ou PNG |
| SVG/SVF (`.svg`, `.svf`) | DXF, PLT ou PNG |

## Uso

```bash
plotconvert desenho.plt
plotconvert desenho.dxf
plotconvert desenho.svg
plotconvert --to svg desenho.dxf
plotconvert --to svg desenho.plt
plotconvert --to plt desenho.svg
plotconvert --to dxf desenho.svg
plotconvert --to png desenho.dxf
```

O formato da entrada é detectado pela extensão e, quando necessário, pelo
conteúdo.

**Conversões suportadas** — use `--to` ou a extensão informada em `--output`
para escolher o formato de saída:

- DXF → PLT, SVG ou PNG;
- PLT/HP-GL → DXF, SVG ou PNG;
- SVG/SVF → DXF, PLT ou PNG.

PNG é exclusivamente **saída**; arquivos `.png` não são aceitos como entrada.

**Saída padrão** (sem `--to` nem extensão explícita em `--output`):

- entrada `.plt` ou `.hpgl` → `.dxf`;
- entrada `.dxf` → `.plt` (HP-GL/2);
- entrada `.svg` ou `.svf` → `.dxf`.

Sem `--output` ou `--output-dir`, o arquivo convertido é criado ao lado da
entrada, com o mesmo nome-base e a nova extensão.

Execute `plotconvert --help` para consultar todas as opções.

## Parâmetros

### `-t, --to <FORMATO>`

Escolhe explicitamente o formato de saída. É a opção principal para selecionar
o destino da conversão.

Valores aceitos:

- `dxf`: gera DXF ASCII R12;
- `svg` ou `svf`: gera SVG;
- `png`: gera imagem PNG rasterizada;
- `plt`: gera PLT usando o valor de `--plt-dialect`;
- `hpgl`: gera PLT em HP-GL clássico;
- `hpgl2`: gera PLT em HP-GL/2.

```bash
# DXF → PLT (HP-GL/2)
plotconvert --to plt desenho.dxf

# DXF → SVG
plotconvert --to svg desenho.dxf

# DXF → PNG
plotconvert --to png desenho.dxf

# PLT → DXF
plotconvert --to dxf desenho.plt

# PLT → SVG
plotconvert --to svg desenho.plt

# PLT → PNG
plotconvert --to png desenho.plt

# SVG → DXF
plotconvert --to dxf desenho.svg

# SVG → PLT (HP-GL clássico)
plotconvert --to hpgl desenho.svg

# SVG → PNG
plotconvert --to png desenho.svg
```

Quando `--to` é usado, ele tem prioridade sobre a extensão informada em
`--output`.

### `-o, --output <ARQUIVO>`

Define o caminho exato do arquivo de saída. Só pode ser usado quando existe
uma única entrada. Quando `--to` não é informado, a extensão da saída escolhe
o formato.

```bash
plotconvert desenho.plt --output resultado.dxf
plotconvert desenho.dxf -o resultado.plt
plotconvert desenho.dxf -o resultado.svg
plotconvert desenho.plt -o resultado.svg
plotconvert desenho.svg -o resultado.dxf
plotconvert desenho.svg -o resultado.plt
plotconvert desenho.dxf -o preview.png
```

Não pode ser combinado com `--output-dir`.

### `-d, --output-dir <DIRETORIO>`

Define o diretório de destino. Pode ser usado com uma ou várias entradas e o
diretório é criado automaticamente quando não existe.

```bash
plotconvert --to svg --output-dir convertidos molde.plt desenho.dxf
```

Cada saída mantém o nome-base da entrada. Por exemplo, `molde.plt` gera
`convertidos/molde.svg` quando usado com `--to svg`.

Para conversões em lote com uma saída específica, use sempre `--to`.

### `--plt-dialect <DIALETO>`

Escolhe o dialeto usado em qualquer conversão cuja saída seja PLT.

Valores aceitos:

- `hpgl2`, `hp-gl2` ou `hp-gl/2`: gera HP-GL/2 com preâmbulo PCL, declaração
  de página, quantidade de canetas, cores e larguras. É o padrão.
- `hpgl` ou `hp-gl`: gera HP-GL clássico, com comandos terminados por `;` e
  sem o preâmbulo HP-GL/2.

```bash
# Saída HP-GL/2, comportamento padrão
plotconvert desenho.dxf
plotconvert --plt-dialect hpgl2 desenho.dxf

# Saída HP-GL clássica
plotconvert --plt-dialect hpgl desenho.dxf
```

Essa opção não altera conversões cuja entrada seja PLT, nem saídas DXF ou SVG.

### `--units-per-mm <NUMERO>`

Define quantas unidades HP-GL representam um milímetro. O padrão é `40`,
equivalente a `1016` unidades por polegada.

Na leitura de PLT, divide as coordenadas HP-GL por esse valor. Na geração de
PLT, multiplica as coordenadas em milímetros por esse valor. Não altera
diretamente a escala de DXF para SVG ou SVG para DXF.

```bash
plotconvert --units-per-mm 40 desenho.plt
plotconvert --units-per-mm 100 desenho.dxf
```

O valor deve ser maior que zero. Normalmente não é necessário alterá-lo.

Não pode ser combinado com `--units-per-inch`.

### `--units-per-inch <NUMERO>`

Define quantas unidades HP-GL representam uma polegada. O padrão implícito é
`1016`, equivalente a `--units-per-mm 40`.

Na leitura de PLT, divide as coordenadas HP-GL por `valor / 25.4`. Na geração
de PLT, multiplica as coordenadas em milímetros por `valor / 25.4`. Não altera
diretamente a escala de DXF para SVG ou SVG para DXF.

```bash
plotconvert --units-per-inch 1016 desenho.plt
plotconvert --units-per-inch 1016 desenho.dxf
plotconvert --units-per-inch 2032 desenho.plt
```

O valor deve ser maior que zero. Use esta opção quando a resolução do plotter
for conhecida em unidades por polegada.

Não pode ser combinado com `--units-per-mm`.

### `--png-dpi <NUMERO>`

Define a resolução da imagem PNG, em pontos por polegada. O padrão é `96`.

Afeta apenas conversões cuja saída seja PNG. Valores maiores geram imagens com
mais pixels e arquivos potencialmente maiores.

```bash
plotconvert --to png desenho.dxf
plotconvert --to png --png-dpi 150 desenho.plt
plotconvert --to png --png-dpi 300 desenho.svg
```

O valor deve ser maior que zero.

### `--png-stroke-scale <NUMERO>`

Multiplica a espessura dos traços na saída PNG. O padrão é `3`, deixando os
contornos mais visíveis na rasterização (traços finos em milímetros viram poucos
pixels em 96 DPI).

Afeta apenas conversões cuja saída seja PNG. A exportação SVG não é alterada.

```bash
plotconvert --to png desenho.dxf
plotconvert --to png --png-stroke-scale 2 desenho.dxf
plotconvert --to png --png-stroke-scale 4 --png-dpi 150 desenho.plt
```

O valor deve ser maior que zero. Use `1` para manter a mesma espessura relativa
da exportação SVG.

### `--png-max-size <PIXELS>`

Limita o **lado maior** (largura ou altura) da imagem PNG, em pixels. A imagem
é reduzida proporcionalmente quando excede esse valor; desenhos já menores **não
são ampliados**.

Afeta apenas conversões cuja saída seja PNG. Útil para gerar thumbnails sem
criar imagens muito grandes a partir de desenhos extensos.

O tamanho base vem de `--png-dpi` e das dimensões do desenho; `--png-max-size`
aplica um teto depois dessa rasterização.

```bash
plotconvert --to png --png-max-size 512 desenho.dxf
plotconvert --to png --png-max-size 256 --png-dpi 96 molde.plt
plotconvert --to png --png-max-size 1024 desenho.svg
```

O valor deve ser um inteiro maior que zero. Sem essa opção, não há limite de
tamanho.

### `--curve-tolerance-mm <MM>`

Define a tolerância, em milímetros, usada para aproximar curvas que não
possuem representação direta no formato de destino. O padrão é `0.05`.

Afeta principalmente `SPLINE`, `ELLIPSE`, curvas Bézier, paths SVG e
círculos/arcos submetidos a transformações não uniformes.

```bash
# Mais precisão e arquivos potencialmente maiores
plotconvert --curve-tolerance-mm 0.01 desenho.dxf

# Menos pontos e arquivos menores
plotconvert --curve-tolerance-mm 0.2 desenho.dxf
```

Valores menores geram mais segmentos. O valor deve ser maior que zero.

### `--normalize-origin`

Move toda a geometria para que o menor X e o menor Y sejam `0,0`, preservando
as dimensões do desenho.

```bash
plotconvert --normalize-origin desenho.dxf
```

É útil para máquinas ou programas que não trabalham bem com coordenadas
negativas ou desenhos afastados da origem.

### `--flip-y`

Inverte o sinal do eixo Y antes de gerar a saída.

```bash
plotconvert --flip-y desenho.plt
plotconvert --flip-y --normalize-origin desenho.dxf
```

Quando combinado com `--normalize-origin`, a inversão é aplicada primeiro e a
geometria resultante é reposicionada em `0,0`.

### `--single-layer`

Aplica-se a qualquer conversão com saída DXF. Coloca todas as entidades na
camada `0`.

```bash
plotconvert --single-layer desenho.plt
```

Sem essa opção, cada caneta ou estilo de traço é exportado para uma camada:
`PEN_001`, `PEN_002`, etc.

### `--strict`

Interrompe a conversão ao encontrar um comando HP-GL, entidade DXF ou elemento
SVG não suportado ou malformado.

```bash
plotconvert --strict desenho.dxf
```

Sem essa opção, o conversor continua processando o restante do arquivo e
mostra avisos em `stderr`. Em DXF, por exemplo, `DIMENSION` e o preenchimento
de `HATCH` são ignorados com aviso.

### `--overwrite`

Permite substituir arquivos de saída existentes.

```bash
plotconvert --overwrite desenho.plt
plotconvert --to svg --output-dir convertidos --overwrite *.dxf
```

Sem essa opção, uma saída existente não é alterada e essa conversão é
reportada como erro.

### `-h, --help`

Mostra a ajuda resumida da linha de comando.

```bash
plotconvert --help
```

### `-V, --version`

Mostra a versão do conversor.

```bash
plotconvert --version
```

### `--`

Encerra o processamento de opções. É necessário para converter um arquivo
cujo nome começa com hífen.

```bash
plotconvert -- -desenho.plt
```

## Exemplos

### DXF → PLT

Converter um DXF para HP-GL/2 (saída padrão):

```bash
plotconvert molde.dxf
plotconvert --to hpgl2 molde.dxf
```

Converter um DXF para HP-GL clássico:

```bash
plotconvert --plt-dialect hpgl molde.dxf
```

### DXF → SVG

```bash
plotconvert --to svg molde.dxf
```

### DXF → PNG

```bash
plotconvert --to png molde.dxf
plotconvert --to png --png-dpi 300 molde.dxf
```

### PLT → DXF

Converter um PLT para DXF R12 (saída padrão):

```bash
plotconvert molde.plt
```

Gerar um DXF simples, sem camadas por caneta:

```bash
plotconvert --single-layer molde.plt
```

### PLT → SVG

```bash
plotconvert --to svg molde.plt
```

### PLT → PNG

```bash
plotconvert --to png molde.plt
```

### SVG → DXF

Converter SVG para DXF (saída padrão):

```bash
plotconvert desenho.svg
plotconvert --to dxf desenho.svg
```

Arquivos com extensão `.svf` também são aceitos como entrada SVG.

### SVG → PLT

```bash
plotconvert --to plt desenho.svg
plotconvert --to hpgl desenho.svf
```

### SVG → PNG

```bash
plotconvert --to png desenho.svg
plotconvert desenho.svg -o preview.png
```

### Opções comuns

Converter vários arquivos em lote:

```bash
plotconvert --to svg --output-dir convertidos molde.plt desenho.dxf bolso.dxf
```

Normalizar a origem, inverter Y e substituir uma saída existente:

```bash
plotconvert --normalize-origin --flip-y --overwrite molde.dxf
```

## Entrada DXF

Saídas possíveis: **PLT** (padrão), **SVG** e **PNG**.

O leitor aceita DXF ASCII R12, R14 e versões posteriores com estrutura por
group codes. São convertidos:

- `LINE`, `ARC`, `CIRCLE`, `POINT`;
- `POLYLINE` e `LWPOLYLINE`, incluindo segmentos com bulge;
- `ELLIPSE` e `SPLINE`, aproximadas por trajetórias;
- `TEXT`, `MTEXT`, `ATTRIB` e `ATTDEF`;
- `SOLID`, `TRACE`, `3DFACE`, `BLOCK` e `INSERT`.

As unidades declaradas em `$INSUNITS` são convertidas para milímetros.
DXFs sem unidade declarada são interpretados como milímetros.

`DIMENSION` é ignorado para não duplicar geometria de cotas. O preenchimento
de `HATCH` também é ignorado; seus contornos originais continuam sendo
convertidos. Use `--strict` para transformar entidades não suportadas em erro.

### DXF → PLT

O padrão de saída é HP-GL/2. Use `--plt-dialect hpgl` para gerar HP-GL
clássico.

### DXF → SVG

`ELLIPSE` e `SPLINE` são aproximadas por segmentos quando necessário. A
precisão é controlada por `--curve-tolerance-mm`. O SVG gerado utiliza
dimensões em milímetros e preserva cores, larguras, canetas, textos, círculos,
arcos e polylines.

### DXF → PNG

Rasteriza o desenho com as mesmas cores e espessuras da exportação SVG. Use
`--png-dpi` para controlar a resolução.

## Entrada PLT/HP-GL

Saídas possíveis: **DXF** (padrão), **SVG** e **PNG**.

São aceitos HP-GL clássico e HP-GL/2, incluindo arquivos com comandos
concatenados, preâmbulos PCL e coordenadas compactadas `PE`. Arquivos com
extensão `.hpgl` são tratados da mesma forma que `.plt`.

### PLT → DXF

O DXF gerado é ASCII R12 e usa milímetros. Por padrão:

- `40` unidades HP-GL equivalem a `1 mm` (`1016` unidades por polegada, padrão);
- canetas são exportadas como camadas `PEN_001`, `PEN_002`, etc.;
- cores e larguras de caneta são preservadas quando declaradas;
- trajetórias viram `POLYLINE`;
- círculos, arcos e textos usam entidades DXF nativas quando possível.

Use `--single-layer` para colocar todas as entidades na camada `0`.

### PLT → SVG

As coordenadas HP-GL são convertidas para milímetros conforme
`--units-per-mm` ou `--units-per-inch`. O escritor SVG preserva cores,
larguras, canetas, textos, círculos, arcos e polylines.

### PLT → PNG

Mesma aparência visual da exportação SVG, convertida para bitmap com fundo
transparente.

## Entrada SVG/SVF

Saídas possíveis: **DXF** (padrão), **PLT** e **PNG**.

Arquivos com extensão `.svf` são aceitos como alias de SVG para compatibilidade
com a grafia indicada, desde que o conteúdo seja XML SVG.

### Leitura SVG

O leitor SVG aceita unidades em `mm`, `cm`, `in`, `pt`, `pc` e pixels CSS a
96 DPI. `width`, `height` e `viewBox` são usados para converter o desenho para
milímetros.

Elementos suportados:

- `line`, `polyline`, `polygon` e `rect`, incluindo cantos arredondados;
- `circle` e `ellipse`;
- `path` com comandos `M`, `L`, `H`, `V`, `C`, `S`, `Q`, `T`, `A` e `Z`,
  absolutos ou relativos;
- `text`;
- grupos `g`, links `a` e `symbol`;
- transformações `matrix`, `translate`, `scale`, `rotate`, `skewX` e `skewY`;
- cores hexadecimais, nomes básicos, `rgb()`, estilos inline e
  `stroke-width`.

Curvas Bézier, arcos elípticos e elipses são aproximados por polylines quando
o destino não oferece uma entidade equivalente. A precisão é controlada por
`--curve-tolerance-mm`.

### SVG → DXF

O DXF gerado é ASCII R12 e usa milímetros. Canetas e estilos de traço viram
camadas `PEN_001`, `PEN_002`, etc., a menos que `--single-layer` seja usado.

### SVG → PLT

Curvas sem equivalente direto em HP-GL são aproximadas por trajetórias. O
padrão de saída é HP-GL/2; use `--plt-dialect hpgl` para HP-GL clássico.

### SVG → PNG

Rasteriza o desenho interpretado a partir do SVG de entrada. Texto depende das
fontes instaladas no sistema.

## Saída PNG

PNG está disponível **somente como formato de saída**, a partir de entradas
DXF, PLT ou SVG.

O conversor gera um SVG intermediário com a mesma lógica de
[`svg_writer.rs`](src/svg_writer.rs) e rasteriza com resvg. A imagem resultante
usa fundo transparente, preserva cores e respeita `--png-dpi` (padrão `96`),
`--png-stroke-scale` (padrão `3`) e, opcionalmente, `--png-max-size` para limitar
o lado maior (ideal para thumbnails).

```bash
plotconvert --to png desenho.dxf
plotconvert --to png --png-dpi 300 molde.plt
plotconvert --to png --png-max-size 512 desenho.dxf
plotconvert desenho.svg -o preview.png
```

## Compilação

```bash
cargo build --release
cargo test
```

O binário Linux será criado em `target/release/plotconvert`. Para Windows,
compile o mesmo projeto para o alvo `x86_64-pc-windows-gnu` ou use o
workflow de release.

Os artefatos prontos são gerados em:

```text
dist/plotconvert-linux-x86_64
dist/plotconvert-windows-x86_64.exe
```

## Licença

Este projeto é distribuído sob a licença [MIT](LICENSE).
