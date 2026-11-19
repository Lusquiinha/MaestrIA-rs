---
name: release-notes
description: Changelog de uma release do maestria-rs. Use ao subir a versão para lançar (`make bump`, "sobe um patch/minor", PR de versão) — escreve o resumo dos PRs desde a última tag em `packaging/changelog/<tag>.md`, que o CI põe no topo das notas da release. Também quando pedirem o changelog de uma release já publicada.
---

# Changelog da release

O job `publicar` do workflow `release` monta as notas da release com `packaging/changelog/<tag>.md` (se existir) seguido do modelo fixo `packaging/NOTAS-DA-RELEASE.md`. O changelog entra no mesmo PR do bump de versão, antes da tag.

## Passos

1. **Intervalo.** A tag nova é `v<versão>` do `Cargo.toml` depois do bump; a anterior, `git describe --tags --abbrev=0 upstream/main`. Pronto quando as duas estão definidas.

2. **O que entrou.** `git log --first-parent --format='%h %s' <anterior>..upstream/main` lista os merges (`Merge pull request #N …`) e os commits feitos direto no `main`. Para cada PR, `gh pr view N -R MatheusPano/MaestrIA-rs --json title,body,files`. Pronto quando cada PR e cada commit direto está classificado: vai para o changelog, ou é só bump de versão / CI / docs internos e fica de fora.

3. **O resumo.** Uma linha por mudança, em português, dizendo o que mudou para quem usa o app — o efeito, não a implementação — terminando no link do PR: `(#48)`. Commit direto sem PR leva o hash curto. Junte PRs que entregam a mesma coisa numa linha só (`(#50, #52)`). Com novidades e correções juntas, separe em `### Novidades` e `### Correções`; com um tipo só, lista simples.

   ```markdown
   ## O que mudou

   - O canvas do Design não treme mais parado (#48)
   ```

4. **Grave e commite.** Salve em `packaging/changelog/<tag>.md` e commite no mesmo PR do bump (`Cargo.toml` + `Cargo.lock`). Pronto quando o PR do bump contém o arquivo; depois do merge, `make tag` publica a release já com o changelog.

## Release já publicada

Quando a tag já subiu sem o arquivo, o changelog vai direto na release. Faça os passos 1–3 com o intervalo `<anterior>..<tag>` e:

- Espere o job `publicar` terminar (`gh run list -R MatheusPano/MaestrIA-rs --workflow release --limit 3`, `gh run watch <id>`): é ele que cria a release, e uma release criada antes faz o CI só anexar os arquivos, sem o texto do modelo.
- Leia o corpo (`gh release view <tag> -R MatheusPano/MaestrIA-rs --json body --jq .body`). Se já começa com `## O que mudou`, troque essa seção (até o próximo `## `); senão, ponha a seção antes do texto. Grave com `gh release edit <tag> -R MatheusPano/MaestrIA-rs --notes-file <arquivo>`. Pronto quando a release mostra a seção no topo e o texto do modelo inteiro abaixo.
- Salve também o resumo em `packaging/changelog/<tag>.md` num PR, para o histórico ficar completo.
