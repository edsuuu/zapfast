# ZapFast pelo apt (fork edsuuu/zapfast)

Este fork publica o próprio `.deb` e um repositório apt em
<https://edsuuu.github.io/zapfast/apt>, assinado com uma chave só dele. O
pacote tem o mesmo nome e os mesmos arquivos do `.deb` oficial e uma versão
maior, então instala por cima dele, e daí em diante as atualizações vêm deste
fork. O ZapFast instalado pelo apt avisa quando sai uma versão nova aqui e
deixa a instalação para o apt.

## Instalar

```bash
curl -fsSL https://edsuuu.github.io/zapfast/apt/zapfast.gpg | sudo tee /usr/share/keyrings/zapfast.gpg > /dev/null
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/zapfast.gpg] https://edsuuu.github.io/zapfast/apt ./" \
  | sudo tee /etc/apt/sources.list.d/zapfast.list
sudo apt update && sudo apt install zapfast
```

Atualizar: `sudo apt update && sudo apt upgrade`. Sem o repositório, o `.deb`
da release também instala direto: `sudo apt install ./zapfast_<versão>_amd64.deb`.

Precisa de Debian 12+, Ubuntu 23.04+ ou derivados (glibc 2.36), amd64.

## Publicar uma versão

1. Suba a versão em `Cargo.toml` (e o `Cargo.lock`), escreva
   `packaging/release-notes/v<versão>.md`, faça o commit e o push no `main`.
2. Rode `packaging/deb/release.sh`, que confere isso e sobe a tag
   `v<versão>`. A tag dispara o workflow **Fork release**
   (`.github/workflows/fork-release.yml`), que:
   - compila em Debian 12 dentro do Docker (`build-deb.sh`, com
     `in-container.sh` e a imagem do `Dockerfile`);
   - cria a release `v<versão>` no GitHub com o `.deb` e as notas;
   - refaz o ramo `gh-pages` com o repositório apt assinado (`publish-apt.sh`).

Não precisa de Docker nem da chave na sua máquina: só do `git` e de acesso de
escrita ao repositório. Para publicar de novo uma tag, rode o workflow pela aba
Actions com a tag no campo `tag`.

Só a última versão fica no repositório apt: o `gh-pages` é refeito do zero.
Para a busca de GIFs vir embutida, crie o segredo `ZAPFAST_GIPHY_KEY` no
repositório; sem ele, cada um pode pôr a própria chave nas Configurações.

## A chave de assinatura

O workflow assina com o segredo `ZAPFAST_APT_SIGNING_KEY` do repositório: a
chave privada em ASCII (armored), sem senha, com o nome
`ZapFast apt (edsuuu/zapfast) <111332935+edsuuu@users.noreply.github.com>`.
A chave atual (impressão digital `8B62E61F44E466880C62E894DDEF75D275C85089`)
foi criada em 27/11/2026, quando a anterior se perdeu.

Para trocar a chave, gere outra e grave o segredo:

```bash
export GNUPGHOME=~/.config/zapfast-apt/gnupg
mkdir -p "$GNUPGHOME" && chmod 700 "$GNUPGHOME"
gpg --batch --passphrase '' --quick-gen-key \
  "ZapFast apt (edsuuu/zapfast) <111332935+edsuuu@users.noreply.github.com>" rsa4096 sign never
gpg --batch --armor --export-secret-keys \
  "ZapFast apt (edsuuu/zapfast) <111332935+edsuuu@users.noreply.github.com>" \
  | gh secret set ZAPFAST_APT_SIGNING_KEY -R edsuuu/zapfast
```

**Guarde uma cópia da pasta `GNUPGHOME`:** o GitHub não devolve o segredo. Com
uma chave nova, cada instalação precisa baixar o `zapfast.gpg` de novo:

```bash
curl -fsSL https://edsuuu.github.io/zapfast/apt/zapfast.gpg | sudo tee /usr/share/keyrings/zapfast.gpg > /dev/null
```

## Testes no mesmo container

`packaging/deb/in-container.sh` roda qualquer comando no ambiente de build, por
exemplo `packaging/deb/in-container.sh cargo test --locked --all-features`.
