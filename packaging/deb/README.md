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
2. Rode `packaging/deb/release.sh`. Ele:
   - compila em Debian 12 dentro do Docker (`in-container.sh`, imagem
     `zapfast-deb-build` do `Dockerfile`, criada na primeira vez);
   - monta `target/deb/dist/zapfast_<versão>_amd64.deb` (`build-deb.sh`);
   - cria a release `v<versão>` no GitHub com o `.deb`;
   - refaz o ramo `gh-pages` com o repositório apt assinado (`publish-apt.sh`)
     e liga o GitHub Pages na primeira vez.

Só a última versão fica no repositório apt: o `gh-pages` é refeito do zero.
Para a busca de GIFs vir embutida, exporte `ZAPFAST_GIPHY_KEY` antes do build;
sem ela, cada um pode pôr a própria chave nas Configurações.

## A chave de assinatura

Fica em `~/.config/zapfast-apt/gnupg` (ou em `ZAPFAST_APT_GNUPGHOME`), fora do
repositório, sem senha; o primeiro `publish-apt.sh` a cria. **Faça backup dessa
pasta.** Sem ela, uma chave nova obriga cada instalação a baixar o
`zapfast.gpg` de novo.

## Testes no mesmo container

`packaging/deb/in-container.sh` roda qualquer comando no ambiente de build, por
exemplo `packaging/deb/in-container.sh cargo test --locked --all-features`.
