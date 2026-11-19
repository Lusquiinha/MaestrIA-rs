---
name: release-notes
description: Changelog de uma release do maestria-rs. Use ao publicar uma versão (`make tag`, "sobe a tag", "solta a release") ou quando pedirem o changelog/"o que mudou" de uma release — resume os PRs desde a tag anterior e coloca o resumo, com link para cada PR, no topo das notas da release no GitHub.
---

# Changelog da release

As notas da release saem do modelo fixo `packaging/NOTAS-DA-RELEASE.md` (instalação, atualizações, primeiro uso), que o workflow `release` publica ao terminar. Esta skill acrescenta no topo delas uma seção `## O que mudou`.

## Passos

1. **Tag e intervalo.** A tag é a pedida, ou a da versão do `Cargo.toml` (`v<versão>`). A anterior: `git describe --tags --abbrev=0 <tag>^`. Pronto quando as duas existem no `upstream`.

2. **O que entrou.** `git log --first-parent --format='%h %s' <anterior>..<tag>` lista os merges (`Merge pull request #N …`) e os commits feitos direto no `main`. Para cada PR, `gh pr view N -R MatheusPano/MaestrIA-rs --json title,body,files`. Pronto quando cada PR e cada commit direto está classificado: vai para o changelog, ou é só bump de versão / CI / docs internos e fica de fora.

3. **O resumo.** Uma linha por mudança, em português, dizendo o que mudou para quem usa o app — o efeito, não a implementação — terminando no link do PR: `(#48)`. Commit direto sem PR leva o hash curto. Junte PRs que entregam a mesma coisa numa linha só (`(#50, #52)`). Com novidades e correções juntas, separe em `### Novidades` e `### Correções`; com um tipo só, lista simples.

   ```markdown
   ## O que mudou

   - O canvas do Design não treme mais parado (#48)
   ```

4. **Espere a release existir.** A release é criada pelo CI, no job `publicar`. Acompanhe com `gh run list -R MatheusPano/MaestrIA-rs --workflow release --limit 3` e `gh run watch <id> -R MatheusPano/MaestrIA-rs`. Pronto quando `gh release view <tag> -R MatheusPano/MaestrIA-rs` responde com o `.deb` e o `.dmg` anexados. Criar ou editar a release antes disso faz o CI só anexar os arquivos e o texto do modelo nunca entra.

5. **Publique.** Leia o corpo atual (`gh release view <tag> --json body --jq .body`). Se ele já começa com `## O que mudou`, troque essa seção (até o próximo `## `); senão, ponha a seção nova antes do texto. Grave com `gh release edit <tag> -R MatheusPano/MaestrIA-rs --notes-file <arquivo>`. Pronto quando `gh release view <tag>` mostra a seção no topo e o texto do modelo inteiro logo abaixo.

6. Devolva o link da release e o resumo publicado.
