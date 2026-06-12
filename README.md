# plt2dxf

Conversor entre HP-GL/HP-GL2 (`.plt`), DXF ASCII e SVG, escrito em Rust.

Conversões disponíveis:

| Entrada | Saída |
| --- | --- |
| DXF | SVG ou PLT |
| PLT/HP-GL | DXF ou SVG |
| SVG/SVF | DXF ou PLT |

## Uso

```bash
plt2dxf desenho.plt
plt2dxf desenho.dxf
plt2dxf desenho.svg
plt2dxf --to svg desenho.dxf
plt2dxf --to plt desenho.svg
plt2dxf --to hpgl desenho.dxf
```

O formato da entrada é detectado pela extensão e, quando necessário, pelo
conteúdo:

- entrada `.plt` ou `.hpgl`: gera um arquivo `.dxf`;
- entrada `.dxf`: gera um arquivo `.plt`.
- entrada `.svg` ou `.svf`: gera um arquivo `.dxf`.

Essas são apenas as saídas padrão. Use `--to` ou uma extensão em `--output`
para escolher outra combinação.

Sem `--output` ou `--output-dir`, o arquivo convertido é criado ao lado da
entrada, com o mesmo nome-base e a nova extensão.

Execute `plt2dxf --help` para consultar todas as opções.

## Parâmetros

### `-t, --to <FORMATO>`

Escolhe explicitamente o formato de saída. É a opção principal para selecionar
entre as seis rotas de conversão.

Valores aceitos:

- `dxf`: gera DXF ASCII R12;
- `svg` ou `svf`: gera SVG;
- `plt`: gera PLT usando o valor de `--plt-dialect`;
- `hpgl`: gera PLT em HP-GL clássico;
- `hpgl2`: gera PLT em HP-GL/2.

```bash
# DXF para SVG
plt2dxf --to svg desenho.dxf

# DXF para PLT HP-GL/2
plt2dxf --to plt desenho.dxf

# PLT para SVG
plt2dxf --to svg desenho.plt

# SVG para DXF
plt2dxf --to dxf desenho.svg

# SVG para HP-GL clássico
plt2dxf --to hpgl desenho.svg
```

Quando `--to` é usado, ele tem prioridade sobre a extensão informada em
`--output`.

### `-o, --output <ARQUIVO>`

Define o caminho exato do arquivo de saída. Só pode ser usado quando existe
uma única entrada. Quando `--to` não é informado, a extensão da saída escolhe
o formato.

```bash
plt2dxf desenho.plt --output resultado.dxf
plt2dxf desenho.dxf -o resultado.plt
plt2dxf desenho.dxf -o resultado.svg
plt2dxf desenho.svg -o resultado.dxf
```

Não pode ser combinado com `--output-dir`.

### `-d, --output-dir <DIRETORIO>`

Define o diretório de destino. Pode ser usado com uma ou várias entradas e o
diretório é criado automaticamente quando não existe.

```bash
plt2dxf --to svg --output-dir convertidos molde.plt desenho.dxf
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
plt2dxf desenho.dxf
plt2dxf --plt-dialect hpgl2 desenho.dxf

# Saída HP-GL clássica
plt2dxf --plt-dialect hpgl desenho.dxf
```

Essa opção não altera conversões cuja entrada seja PLT, nem saídas DXF ou SVG.

### `--units-per-mm <NUMERO>`

Define quantas unidades HP-GL representam um milímetro. O padrão é `40`,
equivalente a 1.016 unidades por polegada.

Na leitura de PLT, divide as coordenadas HP-GL por esse valor. Na geração de
PLT, multiplica as coordenadas em milímetros por esse valor. Não altera
diretamente a escala de DXF para SVG ou SVG para DXF.

```bash
plt2dxf --units-per-mm 40 desenho.plt
plt2dxf --units-per-mm 100 desenho.dxf
```

O valor deve ser maior que zero. Normalmente não é necessário alterá-lo.

### `--curve-tolerance-mm <MM>`

Define a tolerância, em milímetros, usada para aproximar curvas que não
possuem representação direta no formato de destino. O padrão é `0.05`.

Afeta principalmente `SPLINE`, `ELLIPSE`, curvas Bézier, paths SVG e
círculos/arcos submetidos a transformações não uniformes.

```bash
# Mais precisão e arquivos potencialmente maiores
plt2dxf --curve-tolerance-mm 0.01 desenho.dxf

# Menos pontos e arquivos menores
plt2dxf --curve-tolerance-mm 0.2 desenho.dxf
```

Valores menores geram mais segmentos. O valor deve ser maior que zero.

### `--normalize-origin`

Move toda a geometria para que o menor X e o menor Y sejam `0,0`, preservando
as dimensões do desenho.

```bash
plt2dxf --normalize-origin desenho.dxf
```

É útil para máquinas ou programas que não trabalham bem com coordenadas
negativas ou desenhos afastados da origem.

### `--flip-y`

Inverte o sinal do eixo Y antes de gerar a saída.

```bash
plt2dxf --flip-y desenho.plt
plt2dxf --flip-y --normalize-origin desenho.dxf
```

Quando combinado com `--normalize-origin`, a inversão é aplicada primeiro e a
geometria resultante é reposicionada em `0,0`.

### `--single-layer`

Aplica-se a qualquer conversão com saída DXF. Coloca todas as entidades na
camada `0`.

```bash
plt2dxf --single-layer desenho.plt
```

Sem essa opção, cada caneta ou estilo de traço é exportado para uma camada:
`PEN_001`, `PEN_002`, etc.

### `--strict`

Interrompe a conversão ao encontrar um comando HP-GL, entidade DXF ou elemento
SVG não suportado ou malformado.

```bash
plt2dxf --strict desenho.dxf
```

Sem essa opção, o conversor continua processando o restante do arquivo e
mostra avisos em `stderr`. Em DXF, por exemplo, `DIMENSION` e o preenchimento
de `HATCH` são ignorados com aviso.

### `--overwrite`

Permite substituir arquivos de saída existentes.

```bash
plt2dxf --overwrite desenho.plt
plt2dxf --to svg --output-dir convertidos --overwrite *.dxf
```

Sem essa opção, uma saída existente não é alterada e essa conversão é
reportada como erro.

### `-h, --help`

Mostra a ajuda resumida da linha de comando.

```bash
plt2dxf --help
```

### `-V, --version`

Mostra a versão do conversor.

```bash
plt2dxf --version
```

### `--`

Encerra o processamento de opções. É necessário para converter um arquivo
cujo nome começa com hífen.

```bash
plt2dxf -- -desenho.plt
```

## Exemplos

Converter um PLT para DXF R12:

```bash
plt2dxf molde.plt
```

Converter um DXF para HP-GL/2:

```bash
plt2dxf molde.dxf
plt2dxf --to hpgl2 molde.dxf
```

Converter um DXF para HP-GL clássico:

```bash
plt2dxf --plt-dialect hpgl molde.dxf
```

Converter vários arquivos, inclusive em direções diferentes:

```bash
plt2dxf --to svg --output-dir convertidos molde.plt desenho.dxf bolso.dxf
```

Normalizar a origem, inverter Y e substituir uma saída existente:

```bash
plt2dxf --normalize-origin --flip-y --overwrite molde.dxf
```

Gerar um DXF simples, sem camadas por caneta:

```bash
plt2dxf --single-layer molde.plt
```

Converter DXF para SVG:

```bash
plt2dxf --to svg molde.dxf
```

Converter PLT para SVG:

```bash
plt2dxf --to svg molde.plt
```

Converter SVG para DXF:

```bash
plt2dxf --to dxf desenho.svg
```

Converter SVG ou um arquivo com extensão `.svf` para PLT:

```bash
plt2dxf --to plt desenho.svg
plt2dxf --to hpgl desenho.svf
```

## Conversão DXF para PLT

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

O padrão de saída é HP-GL/2. Use `--plt-dialect hpgl` para gerar HP-GL
clássico.

## Conversão PLT para DXF

São aceitos HP-GL clássico e HP-GL/2, incluindo arquivos com comandos
concatenados, preâmbulos PCL e coordenadas compactadas `PE`.

O DXF gerado é ASCII R12 e usa milímetros. Por padrão:

- `40` unidades HP-GL equivalem a `1 mm`;
- canetas são exportadas como camadas `PEN_001`, `PEN_002`, etc.;
- cores e larguras de caneta são preservadas quando declaradas;
- trajetórias viram `POLYLINE`;
- círculos, arcos e textos usam entidades DXF nativas quando possível.

## Conversão SVG

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

O escritor SVG preserva cores, larguras, canetas, textos, círculos, arcos e
polylines. O arquivo gerado utiliza dimensões em milímetros.

Arquivos com extensão `.svf` são aceitos como alias de SVG para compatibilidade
com a grafia indicada, desde que o conteúdo seja XML SVG.

## Compilação

```bash
cargo build --release
cargo test
```

O binário Linux será criado em `target/release/plt2dxf`. Para Windows,
compile o mesmo projeto para o alvo `x86_64-pc-windows-gnu` ou use o
workflow de release.

Os artefatos prontos são gerados em:

```text
dist/plt2dxf-linux-x86_64
dist/plt2dxf-windows-x86_64.exe
```
