use super::input_lexer::Token;
use super::parser::CmdWord;

//
// schema: git add .
// input: git add
//
// match: literal, match: literal
//
// compare: (failure)
// git add "src"
// ___ ___  xxx
// schema: git commit -m "init"
// input: git commit "init"
//
// match:literal, match:literal, missing, match:variable
//
// compare: (failure)
// git commit -m "init"
// ___ ______ xxx _____
//
// schema: display-message -p -t client-0 "hello, world"
// input: display-message -p -t client-0 "hello, world"
//
// compare: (success)
// display-message -p -t client-0 "hello, world"
// --------------- -- -- -------- --------------
//
// schema: display-message -p -t client-0 "hello, world"
// input: display-message -t client-0 "hello, world"
//
// compare: (failure)
// display-message -p -t client-0 "hello, world"
// --------------- xx -- -------- --------------
//
// schema: display-message -p -t client-0 "hello, world"
// input: display-message client-0 "hello, world"
//
// compare: (failure)
// display-message -p -t client-0 "hello, world"
// --------------- xx xx -------- --------------
//
// schema: display-message -p -t client-0 "hello, world"
// input: display-message -p "hello, world"
//
// compare: (failure)
// display-message -p -t client-0 "hello, world"
// --------------- -- xx xxxxxxxx --------------
//
// schema: display-message -p -t client-0 "hello, world"
// input: display-message "hello, world"
//
// compare: (failure)
// display-message -p -t client-0 "hello, world"
// --------------- xx xx xxxxxxxx --------------
//
// edge case which we probably don't want to support.
// schema: display-message -p -t client-0 "hello, world"
// input: display-message -t -p client-0 "hello, world"
//
// compare: (success)
// display-message -t -p client-0 "hello, world"
// --------------- -- -- -------- --------------
///
/// [
///     Match(
///         ast: CmdWord,
///         input: Option<String>,
///         node: CmdWord
///     ),
///     Mismatch(
///         input: Option<String>,
///         node: CmdWord
///     )
/// ]
pub fn match_schema(
    ast: &Vec<CmdWord>,
    tokens: &Vec<Token>,
    ast_idx: usize,
    token_idx: usize,
) -> Vec<(String, bool)> {
    let mut ast_idx = ast_idx;
    let mut token_idx = token_idx;
    let mut res: Vec<(String, bool)> = Vec::new();

    while ast_idx < ast.len() && token_idx < tokens.len() {
        let cmd = &ast[ast_idx];
        let token = &tokens[token_idx];

        match (cmd, token) {
            (CmdWord::Literal { value }, Token::Str(word)) => {
                res.push((cmd.to_string(), value == word));
                ast_idx += 1;
                token_idx += 1;
            }
            (CmdWord::Variable { required, .. }, token) => {
                if let Token::Str(word) = token {
                    res.push((word.clone(), true));
                    ast_idx += 1;
                    token_idx += 1;
                } else {
                    if *required {
                        res.push((cmd.to_string(), false));
                    }
                    ast_idx += 1;
                    token_idx += 1;
                }
            }
            (CmdWord::FlagShort { value, input }, Token::FlagShort(word)) => {
                // TODO: Make an implementation for the flagShort only flag string
                res.push((format!("-{value}"), value == word));
                ast_idx += 1;
                token_idx += 1;

                if let Some(input) = input.as_ref() {
                    match (input, tokens.get(token_idx)) {
                        (CmdWord::Variable { .. }, Some(Token::Str(_))) => {
                            res.push((input.to_string(), true));
                            token_idx += 1;
                        }
                        _ => {
                            res.push((input.to_string(), false));
                        }
                    }
                }
            }
            (CmdWord::FlagLong { value, .. }, Token::FlagLong(word)) => {
                res.push((cmd.to_string(), value == word));
                ast_idx += 1;
                token_idx += 1;
            }
            (CmdWord::FlagCombo { values }, Token::FlagCombo(chars)) => {
                res.push((cmd.to_string(), values == chars));
                ast_idx += 1;
                token_idx += 1;
            }
            (cmd, token) => {
                res.push((cmd.to_string(), false));
                println!(
                    "Cmd and token might be missing impl for {} and {}",
                    cmd, token
                );
                ast_idx += 1;
                token_idx += 1;
            }
        }
    }

    while let Some(cmd) = ast.get(ast_idx) {
        match cmd {
            CmdWord::Variable { required, .. } => {
                res.push((cmd.to_string(), *required));
            }
            _ => {
                res.push((cmd.to_string(), false));
            }
        }
        ast_idx += 1;
    }

    while let Some(token) = tokens.get(token_idx) {
        res.push((token.to_string(), false));
        token_idx += 1;
    }

    return res;
}

mod tests {
    use super::*;
    use crate::ghi::input_lexer::InputCmdLexer;
    use crate::ghi::parser::CmdParser;

    #[test]
    fn match_single_item() {
        let tests = vec![("git", "git"), ("-f", "-f")];

        for (cmd, s) in tests {
            let ast = CmdParser::compile(cmd);
            let input = InputCmdLexer::compile(s);
            let matcher = match_schema(&ast, &input, 0, 0);

            for (val, correctness) in matcher {
                assert_eq!(val, s);
                assert_eq!(correctness, val == s);
            }
        }
    }

    #[test]
    fn match_multiple_items() {
        let val = "git add";
        let ast = CmdParser::compile(&val);
        let input = InputCmdLexer::compile(&val);
        let matcher = match_schema(&ast, &input, 0, 0);
        let splits = val.split_whitespace().collect::<Vec<_>>();

        for (i, (cmd, s)) in matcher.iter().enumerate() {
            let split = splits[i];
            assert_eq!(cmd, split);
            assert_eq!(*s, cmd.as_str() == split);
        }
    }

    #[test]
    fn match_short_flag() {
        let val = "git -f";
        let ast = CmdParser::compile(&val);
        let input = InputCmdLexer::compile(&val);
        let matcher = match_schema(&ast, &input, 0, 0);
        let splits = val.split_whitespace().collect::<Vec<_>>();

        for (i, (cmd, s)) in matcher.iter().enumerate() {
            let split = splits[i];
            assert_eq!(cmd, split);
            assert_eq!(*s, cmd.as_str() == split);
        }
    }

    #[test]
    fn match_long_flag() {
        let val = "git --depht";
        let ast = CmdParser::compile(&val);
        let input = InputCmdLexer::compile(&val);
        let matcher = match_schema(&ast, &input, 0, 0);
        let splits = val.split_whitespace().collect::<Vec<_>>();

        for (i, (cmd, s)) in matcher.iter().enumerate() {
            let split = splits[i];
            assert_eq!(cmd, split);
            assert_eq!(*s, cmd.as_str() == split);
        }
    }

    #[test]
    fn match_combo_flag() {
        let val = "git -la";
        let ast = CmdParser::compile(&val);
        let input = InputCmdLexer::compile(&val);
        let matcher = match_schema(&ast, &input, 0, 0);
        let splits = val.split_whitespace().collect::<Vec<_>>();

        for (i, (cmd, s)) in matcher.iter().enumerate() {
            let split = splits[i];
            assert_eq!(cmd, split);
            assert_eq!(*s, cmd.as_str() == split);
        }
    }

    // #[test]
    // fn match_flag_input() {
    //     let val = "git -l <path>";
    //     let ast = CmdParser::compile(&val);
    //     let input = InputCmdLexer::compile("git -l path");
    //     let matcher = match_schema(&ast, &input, 0, 0);
    //     let splits = val.split_whitespace().collect::<Vec<_>>();
    //
    //     for (i, (cmd, s)) in matcher.iter().enumerate() {
    //         let split = splits[i];
    //        assert_eq!(cmd, split);
    //        assert_eq!(*s, cmd.as_str() == split);
    //     }
    // }
}
