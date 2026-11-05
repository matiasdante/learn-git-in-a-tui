//! Textos de la interfaz en los cuatro idiomas, buscados por el texto en inglés.

use crate::i18n::Lang;

/// (inglés, español, portugués, chino)
const STRINGS: &[(&str, &str, &str, &str)] = &[
    (
        "Sandbox ready. Work in the console, then press F5 to check.",
        "Sandbox listo. Trabajá en la consola y apretá F5 para verificar.",
        "Sandbox pronto. Trabalhe no console e aperte F5 para verificar.",
        "沙盒已就绪。在控制台中操作，然后按 F5 检查。",
    ),
    (
        "✓ Correct! Press F6 to continue.",
        "✓ ¡Correcto! Apretá F6 para continuar.",
        "✓ Correto! Aperte F6 para continuar.",
        "✓ 正确！按 F6 继续。",
    ),
    ("points", "puntos", "pontos", "分"),
    (
        "(sandbox restored to the start of the exercise)",
        "(el sandbox volvió al inicio del ejercicio)",
        "(o sandbox voltou ao início do exercício)",
        "（沙盒已恢复到练习开始时的状态）",
    ),
    (
        "Achievement unlocked",
        "Logro desbloqueado",
        "Conquista desbloqueada",
        "解锁成就",
    ),
    (
        "Free mode: a real git sandbox. Your own git config is never touched.",
        "Modo libre: un sandbox de git real. Tu configuración de git nunca se toca.",
        "Modo livre: um sandbox de git real. Sua configuração de git nunca é tocada.",
        "自由模式：真实的 git 沙盒。你自己的 git 配置绝不会被改动。",
    ),
    (
        "Learn git by practicing real git, inside your terminal",
        "Aprendé git practicando con git real, dentro de tu terminal",
        "Aprenda git praticando com git de verdade, dentro do seu terminal",
        "在终端里用真实的 git 练习，学会 git",
    ),
    (
        "-A sandbox: your own git config is never touched-",
        "-Un sandbox: tu configuración de git nunca se toca-",
        "-Um sandbox: sua configuração de git nunca é tocada-",
        "-沙盒练习：不会改动你自己的 git 配置-",
    ),
    (
        "Take the placement exam",
        "Rendir el examen de ubicación",
        "Fazer o exame de nivelamento",
        "参加分级测试",
    ),
    (
        "Skip it and start at level 1",
        "Omitirlo y empezar en el nivel 1",
        "Pular e começar no nível 1",
        "跳过，从第 1 级开始",
    ),
    (
        "Do you already know some git? A 10-question placement exam picks your starting level.",
        "¿Ya sabés algo de git? Un examen de ubicación de 10 preguntas elige tu nivel de arranque.",
        "Você já sabe algo de git? Um exame de nivelamento com 10 perguntas escolhe seu nível inicial.",
        "你已经懂一些 git 吗？一个包含 10 道题的分级测试会决定你的起始级别。",
    ),
    (
        "Earlier levels always stay available.  ↑/↓ · Enter · L language",
        "Los niveles anteriores siempre quedan disponibles.  ↑/↓ · Enter · L idioma",
        "Os níveis anteriores sempre ficam disponíveis.  ↑/↓ · Enter · L idioma",
        "之前的级别始终可用。  ↑/↓ · Enter · L 语言",
    ),
    (
        " Placement exam ",
        " Examen de ubicación ",
        " Exame de nivelamento ",
        " 分级测试 ",
    ),
    (
        "Exam finished.",
        "Examen terminado.",
        "Exame concluído.",
        "测试完成。",
    ),
    (
        "You start at level",
        "Arrancás en el nivel",
        "Você começa no nível",
        "你从这一级开始：",
    ),
    (
        "Enter: continue",
        "Enter: continuar",
        "Enter: continuar",
        "Enter：继续",
    ),
    ("Question", "Pregunta", "Pergunta", "问题"),
    (
        "↑/↓ or 1-4 to choose · Enter to answer",
        "↑/↓ o 1-4 para elegir · Enter para responder",
        "↑/↓ ou 1-4 para escolher · Enter para responder",
        "↑/↓ 或 1-4 选择 · Enter 回答",
    ),
    (
        "  ← your start",
        "  ← tu inicio",
        "  ← seu início",
        "  ← 你的起点",
    ),
    ("Level", "Nivel", "Nível", "级别"),
    ("Free mode", "Modo libre", "Modo livre", "自由模式"),
    ("Achievements", "Logros", "Conquistas", "成就"),
    (
        "↑/↓ move · Enter open · L: English/Español · Q quit",
        "↑/↓ mover · Enter abrir · L: English/Español · Q salir",
        "↑/↓ mover · Enter abrir · L: idioma · Q sair",
        "↑/↓ 移动 · Enter 打开 · L：语言 · Q 退出",
    ),
    ("Coming soon.", "Próximamente.", "Em breve.", "敬请期待。"),
    (
        "↑/↓ move · Enter open lesson · L language · Esc back",
        "↑/↓ mover · Enter abrir lección · L idioma · Esc volver",
        "↑/↓ mover · Enter abrir lição · L idioma · Esc voltar",
        "↑/↓ 移动 · Enter 打开课程 · L 语言 · Esc 返回",
    ),
    ("Esc: back", "Esc: volver", "Esc: voltar", "Esc：返回"),
    (
        "Explanation only: this machine is missing",
        "Solo explicación: a esta máquina le falta",
        "Somente explicação: falta nesta máquina",
        "仅作讲解：这台机器缺少",
    ),
    (
        "This topic is explained without an exercise.",
        "Este tema se explica sin ejercicio.",
        "Este tema é explicado sem exercício.",
        "这个主题只讲解，没有练习。",
    ),
    (
        "↑/↓ scroll · Enter start exercises · L language · Esc back",
        "↑/↓ desplazar · Enter empezar ejercicios · L idioma · Esc volver",
        "↑/↓ rolar · Enter começar os exercícios · L idioma · Esc voltar",
        "↑/↓ 滚动 · Enter 开始练习 · L 语言 · Esc 返回",
    ),
    (
        "↑/↓ scroll · L language · Esc back",
        "↑/↓ desplazar · L idioma · Esc volver",
        "↑/↓ rolar · L idioma · Esc voltar",
        "↑/↓ 滚动 · L 语言 · Esc 返回",
    ),
    ("Hint", "Pista", "Dica", "提示"),
    ("✓ Correct!", "✓ ¡Correcto!", "✓ Correto!", "✓ 正确！"),
    ("exercise", "ejercicio", "exercício", "练习"),
    (" Console ", " Consola ", " Console ", " 控制台 "),
    (
        "Enter: send to git · Ctrl+D: close stdin · Ctrl+C: stop git",
        "Enter: enviar a git · Ctrl+D: cerrar stdin · Ctrl+C: detener git",
        "Enter: enviar ao git · Ctrl+D: fechar stdin · Ctrl+C: parar o git",
        "Enter：发送给 git · Ctrl+D：关闭 stdin · Ctrl+C：停止 git",
    ),
    (
        "F6: continue · Esc: back to the menu",
        "F6: continuar · Esc: volver al menú",
        "F6: continuar · Esc: voltar ao menu",
        "F6：继续 · Esc：返回菜单",
    ),
    (
        "F5: check · F1: hint · Ctrl+R: restart · Esc: menu · Ctrl+Q: quit",
        "F5: verificar · F1: pista · Ctrl+R: reiniciar · Esc: menú · Ctrl+Q: salir",
        "F5: verificar · F1: dica · Ctrl+R: reiniciar · Esc: menu · Ctrl+Q: sair",
        "F5：检查 · F1：提示 · Ctrl+R：重新开始 · Esc：菜单 · Ctrl+Q：退出",
    ),
    (" Free mode ", " Modo libre ", " Modo livre ", " 自由模式 "),
    (
        "Reset the sandbox? Everything you created will be lost.  y: yes · any other key: no",
        "¿Reiniciar el sandbox? Se pierde todo lo que creaste.  y: sí · otra tecla: no",
        "Reiniciar o sandbox? Tudo o que você criou será perdido.  y: sim · outra tecla: não",
        "重置沙盒？你创建的所有内容都会丢失。  y：是 · 其他键：否",
    ),
    (
        "Enter: run · Ctrl+R: reset sandbox · Esc: menu · PgUp/PgDn: scroll · Ctrl+Q: quit",
        "Enter: ejecutar · Ctrl+R: reiniciar sandbox · Esc: menú · PgUp/PgDn: desplazar · Ctrl+Q: salir",
        "Enter: executar · Ctrl+R: reiniciar o sandbox · Esc: menu · PgUp/PgDn: rolar · Ctrl+Q: sair",
        "Enter：执行 · Ctrl+R：重置沙盒 · Esc：菜单 · PgUp/PgDn：滚动 · Ctrl+Q：退出",
    ),
];

pub fn lookup(lang: Lang, en: &str) -> Option<&'static str> {
    STRINGS
        .iter()
        .find(|(e, ..)| *e == en)
        .map(|(e, es, pt, zh)| match lang {
            Lang::En => *e,
            Lang::Es => *es,
            Lang::Pt => *pt,
            Lang::Zh => *zh,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cada `tr(lang, "en")` del código tiene que estar en la tabla.
    #[test]
    fn every_tr_call_in_the_sources_is_translated() {
        let mut missing = Vec::new();
        let mut calls = 0;
        for source in [include_str!("app.rs"), include_str!("ui.rs")] {
            let mut rest = source;
            while let Some(pos) = rest.find("tr(") {
                // `tr(` precedido por una letra es otra función (por ejemplo `str(`).
                let before = rest[..pos].chars().last();
                rest = &rest[pos + 3..];
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let args = rest.trim_start();
                let Some(after_lang) = args.find(',').map(|i| &args[i + 1..]) else {
                    continue;
                };
                let Some(literal) = first_string_literal(after_lang.trim_start()) else {
                    continue;
                };
                calls += 1;
                if Lang::ALL.iter().any(|l| lookup(*l, &literal).is_none()) {
                    missing.push(literal);
                }
            }
        }
        assert!(calls > 30, "the scan found only {calls} tr() calls");
        assert!(
            missing.is_empty(),
            "UI strings missing in some language: {missing:#?}"
        );
    }

    /// Lee un literal `"..."` al principio de `text` (con escapes simples).
    fn first_string_literal(text: &str) -> Option<String> {
        let mut chars = text.chars();
        if chars.next()? != '"' {
            return None;
        }
        let mut out = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => out.push(match chars.next()? {
                    'n' => '\n',
                    other => other,
                }),
                c => out.push(c),
            }
        }
        None
    }

    #[test]
    fn all_entries_have_text_in_both_languages() {
        for (en, es, pt, zh) in STRINGS {
            assert!(
                !en.is_empty() && !es.is_empty() && !pt.is_empty() && !zh.is_empty(),
                "{en}"
            );
        }
    }
}
