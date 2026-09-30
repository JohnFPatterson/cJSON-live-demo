# Parity report

Result: PASS. 406 fixtures, 406 compared: 406 identical, 0 logged exceptions, 0 diverged; 0 gate problems.

## Method

- Workspace: `/Users/John/cJSON-live-demo/cJSON-live-demo`
- Tree hash: `0e8d373f31f6bb1cf9bfb4b682c7bed068ebe3a0c6906b77646e7d2734167b1f`
- Build: `make oracle rust-driver` (exit 0)
- C oracle: `./build/oracle {input}`
- Rust port: `./target/release/rust-driver {input}`
- Input: fixture path substituted for `{input}`
- Compared: stdout bytes and exit status (stderr not compared)
- Fixture globs: `tests/inputs/*`, `fuzzing/inputs/*`, `tests/json-patch-tests/*.json`, `tests/parity/**/*` (406 files)
- Exceptions file: `PARITY_EXCEPTIONS.md`
- Exception tests: not run (no exception rows)

Reproduce:

```sh
printf '%s' '{"status": "completed", "loop_count": 0, "workspace_roots": ["/Users/John/cJSON-live-demo/cJSON-live-demo"]}' | '/Users/John/.cursor/hooks/c-rust-parity/parity_gate.py' --force
```

## Per-fixture results

| Fixture | Result | C status | Rust status | C stdout sha256 | Rust stdout sha256 |
|---|---|---|---|---|---|
| `fuzzing/inputs/test1` | identical | exit 0 | exit 0 | `c60cf049eeed` | `c60cf049eeed` |
| `fuzzing/inputs/test10` | identical | exit 0 | exit 0 | `ed67fc1770c3` | `ed67fc1770c3` |
| `fuzzing/inputs/test11` | identical | exit 0 | exit 0 | `2ccbe75076f5` | `2ccbe75076f5` |
| `fuzzing/inputs/test2` | identical | exit 0 | exit 0 | `8b4015f05e46` | `8b4015f05e46` |
| `fuzzing/inputs/test3` | identical | exit 0 | exit 0 | `d331c56000df` | `d331c56000df` |
| `fuzzing/inputs/test3.bu` | identical | exit 0 | exit 0 | `fc499ce00cec` | `fc499ce00cec` |
| `fuzzing/inputs/test3.uf` | identical | exit 0 | exit 0 | `c3d5807c79a5` | `c3d5807c79a5` |
| `fuzzing/inputs/test3.uu` | identical | exit 0 | exit 0 | `330f2824347f` | `330f2824347f` |
| `fuzzing/inputs/test4` | identical | exit 0 | exit 0 | `c0db77cdaf46` | `c0db77cdaf46` |
| `fuzzing/inputs/test5` | identical | exit 0 | exit 0 | `e13c63ae50b9` | `e13c63ae50b9` |
| `fuzzing/inputs/test6` | identical | exit 0 | exit 0 | `479b8ac411d9` | `479b8ac411d9` |
| `fuzzing/inputs/test7` | identical | exit 0 | exit 0 | `46e7e665ce7f` | `46e7e665ce7f` |
| `fuzzing/inputs/test8` | identical | exit 0 | exit 0 | `dcaad210223c` | `dcaad210223c` |
| `fuzzing/inputs/test9` | identical | exit 0 | exit 0 | `ab03c82ec70f` | `ab03c82ec70f` |
| `tests/inputs/test1` | identical | exit 0 | exit 0 | `f3939163b224` | `f3939163b224` |
| `tests/inputs/test1.expected` | identical | exit 0 | exit 0 | `0c41eefed175` | `0c41eefed175` |
| `tests/inputs/test10` | identical | exit 0 | exit 0 | `ea293e7fe7bf` | `ea293e7fe7bf` |
| `tests/inputs/test10.expected` | identical | exit 0 | exit 0 | `fa64a0ada803` | `fa64a0ada803` |
| `tests/inputs/test11` | identical | exit 0 | exit 0 | `f15fdff11381` | `f15fdff11381` |
| `tests/inputs/test11.expected` | identical | exit 0 | exit 0 | `b084d85cbf96` | `b084d85cbf96` |
| `tests/inputs/test2` | identical | exit 0 | exit 0 | `0165b5b4e503` | `0165b5b4e503` |
| `tests/inputs/test2.expected` | identical | exit 0 | exit 0 | `97e00eacc4ec` | `97e00eacc4ec` |
| `tests/inputs/test3` | identical | exit 0 | exit 0 | `26e7dff3a7f9` | `26e7dff3a7f9` |
| `tests/inputs/test3.expected` | identical | exit 0 | exit 0 | `a3edb88f5d6d` | `a3edb88f5d6d` |
| `tests/inputs/test4` | identical | exit 0 | exit 0 | `d9a6ba3b86cc` | `d9a6ba3b86cc` |
| `tests/inputs/test4.expected` | identical | exit 0 | exit 0 | `80360cb38400` | `80360cb38400` |
| `tests/inputs/test5` | identical | exit 0 | exit 0 | `28cd1e5d14da` | `28cd1e5d14da` |
| `tests/inputs/test5.expected` | identical | exit 0 | exit 0 | `f20bf4a04f13` | `f20bf4a04f13` |
| `tests/inputs/test6` | identical | exit 0 | exit 0 | `b77d88d5fe1f` | `b77d88d5fe1f` |
| `tests/inputs/test7` | identical | exit 0 | exit 0 | `b196c8ed9e20` | `b196c8ed9e20` |
| `tests/inputs/test7.expected` | identical | exit 0 | exit 0 | `e7f4f56027d9` | `e7f4f56027d9` |
| `tests/inputs/test8` | identical | exit 0 | exit 0 | `107b4589bc34` | `107b4589bc34` |
| `tests/inputs/test8.expected` | identical | exit 0 | exit 0 | `d9332ee0839e` | `d9332ee0839e` |
| `tests/inputs/test9` | identical | exit 0 | exit 0 | `86e8772483ea` | `86e8772483ea` |
| `tests/inputs/test9.expected` | identical | exit 0 | exit 0 | `f12831a16757` | `f12831a16757` |
| `tests/json-patch-tests/cjson-utils-tests.json` | identical | exit 0 | exit 0 | `169de5c8befe` | `169de5c8befe` |
| `tests/json-patch-tests/package.json` | identical | exit 0 | exit 0 | `ebff5c8e8ea4` | `ebff5c8e8ea4` |
| `tests/json-patch-tests/spec_tests.json` | identical | exit 0 | exit 0 | `9543c94fe7d1` | `9543c94fe7d1` |
| `tests/json-patch-tests/tests.json` | identical | exit 0 | exit 0 | `c94ed104071b` | `c94ed104071b` |
| `tests/parity/arr_all_ctrl_ws.json` | identical | exit 0 | exit 0 | `46db0e387dab` | `46db0e387dab` |
| `tests/parity/arr_close_only.json` | identical | exit 0 | exit 0 | `04f08eb68378` | `04f08eb68378` |
| `tests/parity/arr_colon.json` | identical | exit 0 | exit 0 | `3d725a4e44e1` | `3d725a4e44e1` |
| `tests/parity/arr_ctrl_ws.json` | identical | exit 0 | exit 0 | `2c44b319ba72` | `2c44b319ba72` |
| `tests/parity/arr_double_comma.json` | identical | exit 0 | exit 0 | `3ab05f3e2584` | `3ab05f3e2584` |
| `tests/parity/arr_empty.json` | identical | exit 0 | exit 0 | `031b2e4e3e0f` | `031b2e4e3e0f` |
| `tests/parity/arr_empty_ws.json` | identical | exit 0 | exit 0 | `a067d768529b` | `a067d768529b` |
| `tests/parity/arr_empty_ws_all.json` | identical | exit 0 | exit 0 | `c6e7bb10c942` | `c6e7bb10c942` |
| `tests/parity/arr_leading_comma.json` | identical | exit 0 | exit 0 | `9c9fceab91eb` | `9c9fceab91eb` |
| `tests/parity/arr_literals.json` | identical | exit 0 | exit 0 | `700a8e0ce90a` | `700a8e0ce90a` |
| `tests/parity/arr_mismatched.json` | identical | exit 0 | exit 0 | `ba7cdf4ce171` | `ba7cdf4ce171` |
| `tests/parity/arr_missing_comma.json` | identical | exit 0 | exit 0 | `36c8fc4f583c` | `36c8fc4f583c` |
| `tests/parity/arr_mixed.json` | identical | exit 0 | exit 0 | `d916dd00776c` | `d916dd00776c` |
| `tests/parity/arr_nested.json` | identical | exit 0 | exit 0 | `168dd2b3d24e` | `168dd2b3d24e` |
| `tests/parity/arr_nested_objects.json` | identical | exit 0 | exit 0 | `bc5a3b8f5783` | `bc5a3b8f5783` |
| `tests/parity/arr_nested_values.json` | identical | exit 0 | exit 0 | `1a50b4d5e6a6` | `1a50b4d5e6a6` |
| `tests/parity/arr_nul_after_open.json` | identical | exit 0 | exit 0 | `49e2bb322b03` | `49e2bb322b03` |
| `tests/parity/arr_nul_as_ws.json` | identical | exit 0 | exit 0 | `3ec20fb6329d` | `3ec20fb6329d` |
| `tests/parity/arr_only_comma.json` | identical | exit 0 | exit 0 | `ac6282d371c7` | `ac6282d371c7` |
| `tests/parity/arr_single_neg.json` | identical | exit 0 | exit 0 | `5dfbbdfce58c` | `5dfbbdfce58c` |
| `tests/parity/arr_single_number.json` | identical | exit 0 | exit 0 | `91778f423b81` | `91778f423b81` |
| `tests/parity/arr_strings.json` | identical | exit 0 | exit 0 | `f056ec168fb1` | `f056ec168fb1` |
| `tests/parity/arr_trailing_comma.json` | identical | exit 0 | exit 0 | `039d8d1cd8ec` | `039d8d1cd8ec` |
| `tests/parity/arr_unclosed.json` | identical | exit 0 | exit 0 | `7eb5640b3bfc` | `7eb5640b3bfc` |
| `tests/parity/arr_unclosed_after_comma.json` | identical | exit 0 | exit 0 | `e1d2cfe36df7` | `e1d2cfe36df7` |
| `tests/parity/arr_unclosed_empty.json` | identical | exit 0 | exit 0 | `758ba50e41db` | `758ba50e41db` |
| `tests/parity/arr_value_then_garbage.json` | identical | exit 0 | exit 0 | `b1c1dcda0a0d` | `b1c1dcda0a0d` |
| `tests/parity/arr_ws_variants.json` | identical | exit 0 | exit 0 | `1ce23366de4c` | `1ce23366de4c` |
| `tests/parity/big_arr_1000_floats.json` | identical | exit 0 | exit 0 | `d14090acf08b` | `d14090acf08b` |
| `tests/parity/big_arr_1000_ints.json` | identical | exit 0 | exit 0 | `f63df3b29864` | `f63df3b29864` |
| `tests/parity/big_arr_1000_mixed.json` | identical | exit 0 | exit 0 | `2b8adbde19f0` | `2b8adbde19f0` |
| `tests/parity/big_obj_500.json` | identical | exit 0 | exit 0 | `67881a8301e9` | `67881a8301e9` |
| `tests/parity/big_str_4k.json` | identical | exit 0 | exit 0 | `25322d89c19c` | `25322d89c19c` |
| `tests/parity/big_str_4k_escapes.json` | identical | exit 0 | exit 0 | `102436cbc93e` | `102436cbc93e` |
| `tests/parity/big_str_4k_unterminated.json` | identical | exit 0 | exit 0 | `b7c6aff06d5a` | `b7c6aff06d5a` |
| `tests/parity/bom_after_ws.json` | identical | exit 0 | exit 0 | `a48f67712d23` | `a48f67712d23` |
| `tests/parity/bom_arr.json` | identical | exit 0 | exit 0 | `49f210a072fa` | `49f210a072fa` |
| `tests/parity/bom_double.json` | identical | exit 0 | exit 0 | `f4e029b3292b` | `f4e029b3292b` |
| `tests/parity/bom_in_string.json` | identical | exit 0 | exit 0 | `a16083a5ae80` | `a16083a5ae80` |
| `tests/parity/bom_inside_array.json` | identical | exit 0 | exit 0 | `85eafa638fd5` | `85eafa638fd5` |
| `tests/parity/bom_num.json` | identical | exit 0 | exit 0 | `5444e752c82c` | `5444e752c82c` |
| `tests/parity/bom_obj.json` | identical | exit 0 | exit 0 | `7335a71b3944` | `7335a71b3944` |
| `tests/parity/bom_only.json` | identical | exit 0 | exit 0 | `21013463a9b0` | `21013463a9b0` |
| `tests/parity/bom_partial.json` | identical | exit 0 | exit 0 | `d6f35698eaf4` | `d6f35698eaf4` |
| `tests/parity/bom_str.json` | identical | exit 0 | exit 0 | `310b9fdd92e3` | `310b9fdd92e3` |
| `tests/parity/bom_ws.json` | identical | exit 0 | exit 0 | `6662970aadd6` | `6662970aadd6` |
| `tests/parity/bom_ws_then_value.json` | identical | exit 0 | exit 0 | `5ad43436f6a3` | `5ad43436f6a3` |
| `tests/parity/deep_arr_100.json` | identical | exit 0 | exit 0 | `4205bc345ab1` | `4205bc345ab1` |
| `tests/parity/deep_arr_1000.json` | identical | exit 0 | exit 0 | `41dab3406bb1` | `41dab3406bb1` |
| `tests/parity/deep_arr_1000_unclosed.json` | identical | exit 0 | exit 0 | `c376032f2545` | `c376032f2545` |
| `tests/parity/deep_arr_1000_value.json` | identical | exit 0 | exit 0 | `a4ed0b143833` | `a4ed0b143833` |
| `tests/parity/deep_arr_1001.json` | identical | exit 0 | exit 0 | `acc7e7c68d4b` | `acc7e7c68d4b` |
| `tests/parity/deep_arr_1001_value.json` | identical | exit 0 | exit 0 | `ae8d4ab88c63` | `ae8d4ab88c63` |
| `tests/parity/deep_arr_2000.json` | identical | exit 0 | exit 0 | `3b0bada36759` | `3b0bada36759` |
| `tests/parity/deep_arr_999.json` | identical | exit 0 | exit 0 | `ff7560a654df` | `ff7560a654df` |
| `tests/parity/deep_mixed_1000.json` | identical | exit 0 | exit 0 | `58bbeef2422e` | `58bbeef2422e` |
| `tests/parity/deep_mixed_1001.json` | identical | exit 0 | exit 0 | `fca2f6824135` | `fca2f6824135` |
| `tests/parity/deep_obj_1000.json` | identical | exit 0 | exit 0 | `f7f44dcf8024` | `f7f44dcf8024` |
| `tests/parity/deep_obj_1001.json` | identical | exit 0 | exit 0 | `52fab02c3aef` | `52fab02c3aef` |
| `tests/parity/deep_obj_19.json` | identical | exit 0 | exit 0 | `e8e0f0ce574a` | `e8e0f0ce574a` |
| `tests/parity/deep_obj_20.json` | identical | exit 0 | exit 0 | `fc0dad340645` | `fc0dad340645` |
| `tests/parity/deep_obj_999.json` | identical | exit 0 | exit 0 | `e11527993a07` | `e11527993a07` |
| `tests/parity/lit_f.json` | identical | exit 0 | exit 0 | `e219df8ed439` | `e219df8ed439` |
| `tests/parity/lit_fals.json` | identical | exit 0 | exit 0 | `a8fb1b869336` | `a8fb1b869336` |
| `tests/parity/lit_false.json` | identical | exit 0 | exit 0 | `5a9316277d37` | `5a9316277d37` |
| `tests/parity/lit_in_array_nul.json` | identical | exit 0 | exit 0 | `11a8f32c9a61` | `11a8f32c9a61` |
| `tests/parity/lit_in_array_tru.json` | identical | exit 0 | exit 0 | `c0647677fa16` | `c0647677fa16` |
| `tests/parity/lit_in_object_fals.json` | identical | exit 0 | exit 0 | `d6c724568ea6` | `d6c724568ea6` |
| `tests/parity/lit_n.json` | identical | exit 0 | exit 0 | `089c08dc640e` | `089c08dc640e` |
| `tests/parity/lit_nul.json` | identical | exit 0 | exit 0 | `f6da0ec44779` | `f6da0ec44779` |
| `tests/parity/lit_null.json` | identical | exit 0 | exit 0 | `87024dbad6c2` | `87024dbad6c2` |
| `tests/parity/lit_null_null.json` | identical | exit 0 | exit 0 | `7fdffee3472b` | `7fdffee3472b` |
| `tests/parity/lit_null_upper.json` | identical | exit 0 | exit 0 | `9a97d2edb88e` | `9a97d2edb88e` |
| `tests/parity/lit_nullx.json` | identical | exit 0 | exit 0 | `82de3b7a7f89` | `82de3b7a7f89` |
| `tests/parity/lit_t.json` | identical | exit 0 | exit 0 | `86451f94dca2` | `86451f94dca2` |
| `tests/parity/lit_tru.json` | identical | exit 0 | exit 0 | `cff5ca4dda6a` | `cff5ca4dda6a` |
| `tests/parity/lit_true.json` | identical | exit 0 | exit 0 | `3495f56aceb7` | `3495f56aceb7` |
| `tests/parity/lit_true_title.json` | identical | exit 0 | exit 0 | `ba07298ec23e` | `ba07298ec23e` |
| `tests/parity/lit_true_ws.json` | identical | exit 0 | exit 0 | `1144d0531483` | `1144d0531483` |
| `tests/parity/lit_truefalse.json` | identical | exit 0 | exit 0 | `330667e4b634` | `330667e4b634` |
| `tests/parity/lit_undefined.json` | identical | exit 0 | exit 0 | `05604c920bb5` | `05604c920bb5` |
| `tests/parity/min_all_ws.json` | identical | exit 0 | exit 0 | `fa3b6f5da99e` | `fa3b6f5da99e` |
| `tests/parity/min_backslash_backslash_quote.json` | identical | exit 0 | exit 0 | `de4892e336b4` | `de4892e336b4` |
| `tests/parity/min_block_comment.json` | identical | exit 0 | exit 0 | `677774987371` | `677774987371` |
| `tests/parity/min_block_multiline.json` | identical | exit 0 | exit 0 | `772299d30d81` | `772299d30d81` |
| `tests/parity/min_block_only.json` | identical | exit 0 | exit 0 | `c1714eeb92bf` | `c1714eeb92bf` |
| `tests/parity/min_comment_between_tokens.json` | identical | exit 0 | exit 0 | `fd09ecebb699` | `fd09ecebb699` |
| `tests/parity/min_comment_in_key.json` | identical | exit 0 | exit 0 | `e128e72d7885` | `e128e72d7885` |
| `tests/parity/min_comment_only.json` | identical | exit 0 | exit 0 | `0491cbdbb8d7` | `0491cbdbb8d7` |
| `tests/parity/min_crlf_comments.json` | identical | exit 0 | exit 0 | `703009e9205d` | `703009e9205d` |
| `tests/parity/min_empty_block.json` | identical | exit 0 | exit 0 | `6c39831f378b` | `6c39831f378b` |
| `tests/parity/min_escaped_quote_marker.json` | identical | exit 0 | exit 0 | `828f30b56d73` | `828f30b56d73` |
| `tests/parity/min_line_comment.json` | identical | exit 0 | exit 0 | `a1ab25d9e5ca` | `a1ab25d9e5ca` |
| `tests/parity/min_lone_slash.json` | identical | exit 0 | exit 0 | `4bf83f1671c9` | `4bf83f1671c9` |
| `tests/parity/min_nested_block.json` | identical | exit 0 | exit 0 | `a105a3423e7b` | `a105a3423e7b` |
| `tests/parity/min_nul_then_more.json` | identical | exit 0 | exit 0 | `884a090920c8` | `884a090920c8` |
| `tests/parity/min_slash_eof.json` | identical | exit 0 | exit 0 | `6b5919d3b0d4` | `6b5919d3b0d4` |
| `tests/parity/min_slash_in_string_end.json` | identical | exit 0 | exit 0 | `6efd4176b478` | `6efd4176b478` |
| `tests/parity/min_slash_star_slash.json` | identical | exit 0 | exit 0 | `bea5d5078b1a` | `bea5d5078b1a` |
| `tests/parity/min_star_eof.json` | identical | exit 0 | exit 0 | `da1f3ec910c1` | `da1f3ec910c1` |
| `tests/parity/min_string_backslash_eof.json` | identical | exit 0 | exit 0 | `63033fcf9622` | `63033fcf9622` |
| `tests/parity/min_string_comment_markers.json` | identical | exit 0 | exit 0 | `c34dcc30071d` | `c34dcc30071d` |
| `tests/parity/min_unterminated_block.json` | identical | exit 0 | exit 0 | `73cdb2c79e94` | `73cdb2c79e94` |
| `tests/parity/min_unterminated_line.json` | identical | exit 0 | exit 0 | `f10c7e239017` | `f10c7e239017` |
| `tests/parity/min_unterminated_string.json` | identical | exit 0 | exit 0 | `d53aaa8c5a2c` | `d53aaa8c5a2c` |
| `tests/parity/min_ws_inside_string.json` | identical | exit 0 | exit 0 | `b24ef266c132` | `b24ef266c132` |
| `tests/parity/mut_000.json` | identical | exit 0 | exit 0 | `c607df866f55` | `c607df866f55` |
| `tests/parity/mut_001.json` | identical | exit 0 | exit 0 | `9d3cf973c779` | `9d3cf973c779` |
| `tests/parity/mut_002.json` | identical | exit 0 | exit 0 | `b662828f2634` | `b662828f2634` |
| `tests/parity/mut_003.json` | identical | exit 0 | exit 0 | `043d0ef466a6` | `043d0ef466a6` |
| `tests/parity/mut_004.json` | identical | exit 0 | exit 0 | `5477535e3e13` | `5477535e3e13` |
| `tests/parity/mut_005.json` | identical | exit 0 | exit 0 | `1884d0607fcc` | `1884d0607fcc` |
| `tests/parity/mut_006.json` | identical | exit 0 | exit 0 | `da529805eb04` | `da529805eb04` |
| `tests/parity/mut_007.json` | identical | exit 0 | exit 0 | `d093f5e7c9fa` | `d093f5e7c9fa` |
| `tests/parity/mut_008.json` | identical | exit 0 | exit 0 | `f14211706959` | `f14211706959` |
| `tests/parity/mut_009.json` | identical | exit 0 | exit 0 | `1dbaff241ffb` | `1dbaff241ffb` |
| `tests/parity/mut_010.json` | identical | exit 0 | exit 0 | `e954823f1d5e` | `e954823f1d5e` |
| `tests/parity/mut_011.json` | identical | exit 0 | exit 0 | `f4cb722c90e7` | `f4cb722c90e7` |
| `tests/parity/mut_012.json` | identical | exit 0 | exit 0 | `7453ffdb0085` | `7453ffdb0085` |
| `tests/parity/mut_013.json` | identical | exit 0 | exit 0 | `1f3d07856dce` | `1f3d07856dce` |
| `tests/parity/mut_014.json` | identical | exit 0 | exit 0 | `e8cdefd04bd0` | `e8cdefd04bd0` |
| `tests/parity/mut_015.json` | identical | exit 0 | exit 0 | `40b6e99d2796` | `40b6e99d2796` |
| `tests/parity/mut_016.json` | identical | exit 0 | exit 0 | `ae0f68bae707` | `ae0f68bae707` |
| `tests/parity/mut_017.json` | identical | exit 0 | exit 0 | `5863438710c5` | `5863438710c5` |
| `tests/parity/mut_018.json` | identical | exit 0 | exit 0 | `015a3d007020` | `015a3d007020` |
| `tests/parity/mut_019.json` | identical | exit 0 | exit 0 | `00481c462d18` | `00481c462d18` |
| `tests/parity/mut_020.json` | identical | exit 0 | exit 0 | `f6ca47156f96` | `f6ca47156f96` |
| `tests/parity/mut_021.json` | identical | exit 0 | exit 0 | `4fe91e156d4b` | `4fe91e156d4b` |
| `tests/parity/mut_022.json` | identical | exit 0 | exit 0 | `739b1c9419a9` | `739b1c9419a9` |
| `tests/parity/mut_023.json` | identical | exit 0 | exit 0 | `3ed5c1a453d7` | `3ed5c1a453d7` |
| `tests/parity/mut_024.json` | identical | exit 0 | exit 0 | `f99e614bb964` | `f99e614bb964` |
| `tests/parity/mut_025.json` | identical | exit 0 | exit 0 | `e60f01331f3e` | `e60f01331f3e` |
| `tests/parity/mut_026.json` | identical | exit 0 | exit 0 | `d6350973c201` | `d6350973c201` |
| `tests/parity/mut_027.json` | identical | exit 0 | exit 0 | `281c16edd1f3` | `281c16edd1f3` |
| `tests/parity/mut_028.json` | identical | exit 0 | exit 0 | `d93a570bc120` | `d93a570bc120` |
| `tests/parity/mut_029.json` | identical | exit 0 | exit 0 | `5ca3dfb69ead` | `5ca3dfb69ead` |
| `tests/parity/num_0_000001.json` | identical | exit 0 | exit 0 | `c794ad4ea307` | `c794ad4ea307` |
| `tests/parity/num_0_1.json` | identical | exit 0 | exit 0 | `69ca2161c5d4` | `69ca2161c5d4` |
| `tests/parity/num_0_1e1.json` | identical | exit 0 | exit 0 | `1721a96826c5` | `1721a96826c5` |
| `tests/parity/num_0_3.json` | identical | exit 0 | exit 0 | `68f2126262d4` | `68f2126262d4` |
| `tests/parity/num_0_30000000000000004.json` | identical | exit 0 | exit 0 | `bc48f569e185` | `bc48f569e185` |
| `tests/parity/num_100.json` | identical | exit 0 | exit 0 | `0205c1e0220e` | `0205c1e0220e` |
| `tests/parity/num_123_456e_minus789.json` | identical | exit 0 | exit 0 | `69f06a577245` | `69f06a577245` |
| `tests/parity/num_12_5.json` | identical | exit 0 | exit 0 | `4d40a8018c25` | `4d40a8018c25` |
| `tests/parity/num_1E2_plus.json` | identical | exit 0 | exit 0 | `cd5f13bd5e2b` | `cd5f13bd5e2b` |
| `tests/parity/num_1_0.json` | identical | exit 0 | exit 0 | `227e70d8c059` | `227e70d8c059` |
| `tests/parity/num_1_5e300.json` | identical | exit 0 | exit 0 | `660b7e1d0150` | `660b7e1d0150` |
| `tests/parity/num_1e15.json` | identical | exit 0 | exit 0 | `c2b87a4495e4` | `c2b87a4495e4` |
| `tests/parity/num_1e16.json` | identical | exit 0 | exit 0 | `d17f1f31f691` | `d17f1f31f691` |
| `tests/parity/num_1e17.json` | identical | exit 0 | exit 0 | `8a2e653d94c1` | `8a2e653d94c1` |
| `tests/parity/num_1e2.json` | identical | exit 0 | exit 0 | `c5289f6326e8` | `c5289f6326e8` |
| `tests/parity/num_1e20.json` | identical | exit 0 | exit 0 | `2165fbf55e78` | `2165fbf55e78` |
| `tests/parity/num_1e21.json` | identical | exit 0 | exit 0 | `248c1d4142cb` | `248c1d4142cb` |
| `tests/parity/num_1e308.json` | identical | exit 0 | exit 0 | `d50b6747eeaf` | `d50b6747eeaf` |
| `tests/parity/num_1e309.json` | identical | exit 0 | exit 0 | `555846c840cf` | `555846c840cf` |
| `tests/parity/num_1e_minus2.json` | identical | exit 0 | exit 0 | `f1fe51532239` | `f1fe51532239` |
| `tests/parity/num_1e_minus400.json` | identical | exit 0 | exit 0 | `e586352a85d3` | `e586352a85d3` |
| `tests/parity/num_1e_minus5.json` | identical | exit 0 | exit 0 | `8f82491a86da` | `8f82491a86da` |
| `tests/parity/num_1e_minus7.json` | identical | exit 0 | exit 0 | `db2f6c03e111` | `db2f6c03e111` |
| `tests/parity/num_2_5.json` | identical | exit 0 | exit 0 | `8f2c6b25d4c0` | `8f2c6b25d4c0` |
| `tests/parity/num_big_int_18.json` | identical | exit 0 | exit 0 | `cf19d9c8ce2c` | `cf19d9c8ce2c` |
| `tests/parity/num_big_int_40.json` | identical | exit 0 | exit 0 | `088d0523d666` | `088d0523d666` |
| `tests/parity/num_big_int_80.json` | identical | exit 0 | exit 0 | `4063f87b9d71` | `4063f87b9d71` |
| `tests/parity/num_comma_decimal.json` | identical | exit 0 | exit 0 | `8bf85bb4a4d0` | `8bf85bb4a4d0` |
| `tests/parity/num_dbl_max.json` | identical | exit 0 | exit 0 | `0ded975257f3` | `0ded975257f3` |
| `tests/parity/num_dbl_min.json` | identical | exit 0 | exit 0 | `766fdd9ed709` | `766fdd9ed709` |
| `tests/parity/num_denorm_min.json` | identical | exit 0 | exit 0 | `8adbd54253fd` | `8adbd54253fd` |
| `tests/parity/num_digits_17.json` | identical | exit 0 | exit 0 | `c541769b34b2` | `c541769b34b2` |
| `tests/parity/num_dot_5.json` | identical | exit 0 | exit 0 | `e7a5b26de8c6` | `e7a5b26de8c6` |
| `tests/parity/num_dot_5_neg.json` | identical | exit 0 | exit 0 | `d20f18f0c63c` | `d20f18f0c63c` |
| `tests/parity/num_double_exp.json` | identical | exit 0 | exit 0 | `9ddde29f790e` | `9ddde29f790e` |
| `tests/parity/num_exp_dot.json` | identical | exit 0 | exit 0 | `2722b0629dfb` | `2722b0629dfb` |
| `tests/parity/num_exp_empty.json` | identical | exit 0 | exit 0 | `80628a0279b9` | `80628a0279b9` |
| `tests/parity/num_exp_minus_empty.json` | identical | exit 0 | exit 0 | `0b34e8d96296` | `0b34e8d96296` |
| `tests/parity/num_exp_plus_empty.json` | identical | exit 0 | exit 0 | `dc90b6a6e4cb` | `dc90b6a6e4cb` |
| `tests/parity/num_frac_near_int.json` | identical | exit 0 | exit 0 | `f4d484752a56` | `f4d484752a56` |
| `tests/parity/num_hex.json` | identical | exit 0 | exit 0 | `97c64bb53ec4` | `97c64bb53ec4` |
| `tests/parity/num_huge_exp.json` | identical | exit 0 | exit 0 | `01d187134f95` | `01d187134f95` |
| `tests/parity/num_in_array.json` | identical | exit 0 | exit 0 | `0d31e566fac1` | `0d31e566fac1` |
| `tests/parity/num_inf_word.json` | identical | exit 0 | exit 0 | `7a2071d618ec` | `7a2071d618ec` |
| `tests/parity/num_infinity_word.json` | identical | exit 0 | exit 0 | `05e3d330a489` | `05e3d330a489` |
| `tests/parity/num_int_as_float.json` | identical | exit 0 | exit 0 | `159954b1798c` | `159954b1798c` |
| `tests/parity/num_int_max.json` | identical | exit 0 | exit 0 | `c51c5e9e99a4` | `c51c5e9e99a4` |
| `tests/parity/num_int_max_plus1.json` | identical | exit 0 | exit 0 | `7656ac6e5c06` | `7656ac6e5c06` |
| `tests/parity/num_int_min.json` | identical | exit 0 | exit 0 | `ca6de0dc00e9` | `ca6de0dc00e9` |
| `tests/parity/num_int_min_minus1.json` | identical | exit 0 | exit 0 | `4399a10d2de1` | `4399a10d2de1` |
| `tests/parity/num_leading_zero.json` | identical | exit 0 | exit 0 | `92c0f730f42e` | `92c0f730f42e` |
| `tests/parity/num_leading_zeros.json` | identical | exit 0 | exit 0 | `18ce378877c5` | `18ce378877c5` |
| `tests/parity/num_long_mantissa_70.json` | identical | exit 0 | exit 0 | `04bde92a4a04` | `04bde92a4a04` |
| `tests/parity/num_max_safe_frac.json` | identical | exit 0 | exit 0 | `e1b519264f77` | `e1b519264f77` |
| `tests/parity/num_minus_minus_one.json` | identical | exit 0 | exit 0 | `0f38241985db` | `0f38241985db` |
| `tests/parity/num_minus_only.json` | identical | exit 0 | exit 0 | `5cd4b8868e36` | `5cd4b8868e36` |
| `tests/parity/num_nan_word.json` | identical | exit 0 | exit 0 | `63e61dc838ce` | `63e61dc838ce` |
| `tests/parity/num_neg_12_5e_minus3.json` | identical | exit 0 | exit 0 | `2d4a15ab6660` | `2d4a15ab6660` |
| `tests/parity/num_neg_1e309.json` | identical | exit 0 | exit 0 | `e771de67391a` | `e771de67391a` |
| `tests/parity/num_neg_frac.json` | identical | exit 0 | exit 0 | `38357debc668` | `38357debc668` |
| `tests/parity/num_neg_infinity_word.json` | identical | exit 0 | exit 0 | `6d164f20465a` | `6d164f20465a` |
| `tests/parity/num_neg_one.json` | identical | exit 0 | exit 0 | `8a255210404f` | `8a255210404f` |
| `tests/parity/num_neg_zero.json` | identical | exit 0 | exit 0 | `02db1ddd42e8` | `02db1ddd42e8` |
| `tests/parity/num_neg_zero_frac.json` | identical | exit 0 | exit 0 | `c9ae86a0a6f2` | `c9ae86a0a6f2` |
| `tests/parity/num_one.json` | identical | exit 0 | exit 0 | `1e8dad23c379` | `1e8dad23c379` |
| `tests/parity/num_padded.json` | identical | exit 0 | exit 0 | `f48764d78a29` | `f48764d78a29` |
| `tests/parity/num_pi.json` | identical | exit 0 | exit 0 | `cf91af7f135d` | `cf91af7f135d` |
| `tests/parity/num_pi_long.json` | identical | exit 0 | exit 0 | `d73ceed061e4` | `d73ceed061e4` |
| `tests/parity/num_plus_one.json` | identical | exit 0 | exit 0 | `bc266a5a8f9f` | `bc266a5a8f9f` |
| `tests/parity/num_space_inside.json` | identical | exit 0 | exit 0 | `aa92d28d4830` | `aa92d28d4830` |
| `tests/parity/num_third.json` | identical | exit 0 | exit 0 | `8e43baad0d85` | `8e43baad0d85` |
| `tests/parity/num_tiny_exp.json` | identical | exit 0 | exit 0 | `7d521d6279eb` | `7d521d6279eb` |
| `tests/parity/num_trailing_dot.json` | identical | exit 0 | exit 0 | `ef67504d0bcf` | `ef67504d0bcf` |
| `tests/parity/num_trailing_letters.json` | identical | exit 0 | exit 0 | `06895d7d2f8d` | `06895d7d2f8d` |
| `tests/parity/num_two_dots.json` | identical | exit 0 | exit 0 | `47c433b12f2f` | `47c433b12f2f` |
| `tests/parity/num_two_pow_53.json` | identical | exit 0 | exit 0 | `34ed054fab7b` | `34ed054fab7b` |
| `tests/parity/num_two_pow_53_plus1.json` | identical | exit 0 | exit 0 | `efc7ca84a45c` | `efc7ca84a45c` |
| `tests/parity/num_uint64_max.json` | identical | exit 0 | exit 0 | `77d467e5c489` | `77d467e5c489` |
| `tests/parity/num_upper_E5.json` | identical | exit 0 | exit 0 | `61f01bd1b47d` | `61f01bd1b47d` |
| `tests/parity/num_valueint_saturate.json` | identical | exit 0 | exit 0 | `f1d81bfc0743` | `f1d81bfc0743` |
| `tests/parity/num_zero.json` | identical | exit 0 | exit 0 | `b1f7c8cc716d` | `b1f7c8cc716d` |
| `tests/parity/obj_all_types.json` | identical | exit 0 | exit 0 | `845befb05ab5` | `845befb05ab5` |
| `tests/parity/obj_array_value_garbage.json` | identical | exit 0 | exit 0 | `98bac142c857` | `98bac142c857` |
| `tests/parity/obj_bare_key.json` | identical | exit 0 | exit 0 | `5f9a9309efc2` | `5f9a9309efc2` |
| `tests/parity/obj_case_variant_keys.json` | identical | exit 0 | exit 0 | `57756e2e4957` | `57756e2e4957` |
| `tests/parity/obj_case_variant_keys_3.json` | identical | exit 0 | exit 0 | `ade7b913a473` | `ade7b913a473` |
| `tests/parity/obj_case_variant_values.json` | identical | exit 0 | exit 0 | `d041a1ff6e5b` | `d041a1ff6e5b` |
| `tests/parity/obj_close_only.json` | identical | exit 0 | exit 0 | `cb15e6ff9c6a` | `cb15e6ff9c6a` |
| `tests/parity/obj_colon_only.json` | identical | exit 0 | exit 0 | `37b6e9a6cdbf` | `37b6e9a6cdbf` |
| `tests/parity/obj_comma_first.json` | identical | exit 0 | exit 0 | `2c16110c522f` | `2c16110c522f` |
| `tests/parity/obj_ctrl_ws.json` | identical | exit 0 | exit 0 | `887f94ed317c` | `887f94ed317c` |
| `tests/parity/obj_double_colon.json` | identical | exit 0 | exit 0 | `536e570e6834` | `536e570e6834` |
| `tests/parity/obj_duplicate_keys.json` | identical | exit 0 | exit 0 | `77bee59807e3` | `77bee59807e3` |
| `tests/parity/obj_duplicate_keys_nested.json` | identical | exit 0 | exit 0 | `6a7328ee3dcc` | `6a7328ee3dcc` |
| `tests/parity/obj_empty.json` | identical | exit 0 | exit 0 | `110b8994fabe` | `110b8994fabe` |
| `tests/parity/obj_empty_key.json` | identical | exit 0 | exit 0 | `13268332b73c` | `13268332b73c` |
| `tests/parity/obj_empty_ws.json` | identical | exit 0 | exit 0 | `f8765f3102b0` | `f8765f3102b0` |
| `tests/parity/obj_escaped_key.json` | identical | exit 0 | exit 0 | `f04cef4e0988` | `f04cef4e0988` |
| `tests/parity/obj_key_bad_escape.json` | identical | exit 0 | exit 0 | `6c9991a4f06c` | `6c9991a4f06c` |
| `tests/parity/obj_key_only.json` | identical | exit 0 | exit 0 | `c80f946257b7` | `c80f946257b7` |
| `tests/parity/obj_key_u0000.json` | identical | exit 0 | exit 0 | `8381f4614f35` | `8381f4614f35` |
| `tests/parity/obj_many_keys.json` | identical | exit 0 | exit 0 | `44cf552400fe` | `44cf552400fe` |
| `tests/parity/obj_mismatched.json` | identical | exit 0 | exit 0 | `343edda86701` | `343edda86701` |
| `tests/parity/obj_missing_colon.json` | identical | exit 0 | exit 0 | `130f6ef2a9c7` | `130f6ef2a9c7` |
| `tests/parity/obj_missing_comma.json` | identical | exit 0 | exit 0 | `8ca370ba6487` | `8ca370ba6487` |
| `tests/parity/obj_missing_value.json` | identical | exit 0 | exit 0 | `59680ee56f53` | `59680ee56f53` |
| `tests/parity/obj_nested.json` | identical | exit 0 | exit 0 | `8f98b46d90b5` | `8f98b46d90b5` |
| `tests/parity/obj_non_string_key.json` | identical | exit 0 | exit 0 | `ba164b3a177e` | `ba164b3a177e` |
| `tests/parity/obj_null_key.json` | identical | exit 0 | exit 0 | `a2f78144438e` | `a2f78144438e` |
| `tests/parity/obj_simple.json` | identical | exit 0 | exit 0 | `4382af961844` | `4382af961844` |
| `tests/parity/obj_single_quote_key.json` | identical | exit 0 | exit 0 | `d11b9576abe7` | `d11b9576abe7` |
| `tests/parity/obj_trailing_comma.json` | identical | exit 0 | exit 0 | `6496a3fbf45b` | `6496a3fbf45b` |
| `tests/parity/obj_unclosed.json` | identical | exit 0 | exit 0 | `1135f28f916d` | `1135f28f916d` |
| `tests/parity/obj_unclosed_after_comma.json` | identical | exit 0 | exit 0 | `42059ea91d0a` | `42059ea91d0a` |
| `tests/parity/obj_unclosed_empty.json` | identical | exit 0 | exit 0 | `2cf83843d9dd` | `2cf83843d9dd` |
| `tests/parity/obj_unclosed_key.json` | identical | exit 0 | exit 0 | `e7dd2edaed61` | `e7dd2edaed61` |
| `tests/parity/obj_ws.json` | identical | exit 0 | exit 0 | `b046370db79c` | `b046370db79c` |
| `tests/parity/rnd_000.json` | identical | exit 0 | exit 0 | `ebdf4bb385bb` | `ebdf4bb385bb` |
| `tests/parity/rnd_001.json` | identical | exit 0 | exit 0 | `8cbf2934b072` | `8cbf2934b072` |
| `tests/parity/rnd_002.json` | identical | exit 0 | exit 0 | `a79ff6ffebbb` | `a79ff6ffebbb` |
| `tests/parity/rnd_003.json` | identical | exit 0 | exit 0 | `b8eb9cb2207b` | `b8eb9cb2207b` |
| `tests/parity/rnd_004.json` | identical | exit 0 | exit 0 | `8bb417859818` | `8bb417859818` |
| `tests/parity/rnd_005.json` | identical | exit 0 | exit 0 | `ed3e5f08c2aa` | `ed3e5f08c2aa` |
| `tests/parity/rnd_006.json` | identical | exit 0 | exit 0 | `3bc1752eae91` | `3bc1752eae91` |
| `tests/parity/rnd_007.json` | identical | exit 0 | exit 0 | `18f01ec87884` | `18f01ec87884` |
| `tests/parity/rnd_008.json` | identical | exit 0 | exit 0 | `b688a616aa1b` | `b688a616aa1b` |
| `tests/parity/rnd_009.json` | identical | exit 0 | exit 0 | `83dbed63a8df` | `83dbed63a8df` |
| `tests/parity/rnd_010.json` | identical | exit 0 | exit 0 | `646ab4f8ec0b` | `646ab4f8ec0b` |
| `tests/parity/rnd_011.json` | identical | exit 0 | exit 0 | `77e8e2b471a1` | `77e8e2b471a1` |
| `tests/parity/rnd_012.json` | identical | exit 0 | exit 0 | `625544e1cd30` | `625544e1cd30` |
| `tests/parity/rnd_013.json` | identical | exit 0 | exit 0 | `6ab0ce5557c9` | `6ab0ce5557c9` |
| `tests/parity/rnd_014.json` | identical | exit 0 | exit 0 | `9991b850ca15` | `9991b850ca15` |
| `tests/parity/rnd_015.json` | identical | exit 0 | exit 0 | `511c92dc4661` | `511c92dc4661` |
| `tests/parity/rnd_016.json` | identical | exit 0 | exit 0 | `aacc8f0e914c` | `aacc8f0e914c` |
| `tests/parity/rnd_017.json` | identical | exit 0 | exit 0 | `c2d9b57621a8` | `c2d9b57621a8` |
| `tests/parity/rnd_018.json` | identical | exit 0 | exit 0 | `3f5779449404` | `3f5779449404` |
| `tests/parity/rnd_019.json` | identical | exit 0 | exit 0 | `9039a1ca28de` | `9039a1ca28de` |
| `tests/parity/rnd_020.json` | identical | exit 0 | exit 0 | `70292b96c071` | `70292b96c071` |
| `tests/parity/rnd_021.json` | identical | exit 0 | exit 0 | `f14258d17577` | `f14258d17577` |
| `tests/parity/rnd_022.json` | identical | exit 0 | exit 0 | `aa9b9a0a8c99` | `aa9b9a0a8c99` |
| `tests/parity/rnd_023.json` | identical | exit 0 | exit 0 | `385013c9680d` | `385013c9680d` |
| `tests/parity/rnd_024.json` | identical | exit 0 | exit 0 | `6a17657e45ba` | `6a17657e45ba` |
| `tests/parity/rnd_025.json` | identical | exit 0 | exit 0 | `27f67d35ca1c` | `27f67d35ca1c` |
| `tests/parity/rnd_026.json` | identical | exit 0 | exit 0 | `0d941836da28` | `0d941836da28` |
| `tests/parity/rnd_027.json` | identical | exit 0 | exit 0 | `567ed090898d` | `567ed090898d` |
| `tests/parity/rnd_028.json` | identical | exit 0 | exit 0 | `36b45d540bd0` | `36b45d540bd0` |
| `tests/parity/rnd_029.json` | identical | exit 0 | exit 0 | `55c9a29419a7` | `55c9a29419a7` |
| `tests/parity/str_all_control_escaped.json` | identical | exit 0 | exit 0 | `1d5af643c82b` | `1d5af643c82b` |
| `tests/parity/str_all_escapes.json` | identical | exit 0 | exit 0 | `b4d1380e5100` | `b4d1380e5100` |
| `tests/parity/str_backslash_eof.json` | identical | exit 0 | exit 0 | `63033fcf9622` | `63033fcf9622` |
| `tests/parity/str_backslash_only.json` | identical | exit 0 | exit 0 | `3d88045fdf48` | `3d88045fdf48` |
| `tests/parity/str_bad_hex.json` | identical | exit 0 | exit 0 | `045a59837a00` | `045a59837a00` |
| `tests/parity/str_bad_hex_first.json` | identical | exit 0 | exit 0 | `613d40200b53` | `613d40200b53` |
| `tests/parity/str_empty.json` | identical | exit 0 | exit 0 | `a436b273803e` | `a436b273803e` |
| `tests/parity/str_escaped_backslash_end.json` | identical | exit 0 | exit 0 | `b933aa9613c5` | `b933aa9613c5` |
| `tests/parity/str_escaped_quote.json` | identical | exit 0 | exit 0 | `a132ffdb329c` | `a132ffdb329c` |
| `tests/parity/str_high_then_escape_n.json` | identical | exit 0 | exit 0 | `a692fc8943d2` | `a692fc8943d2` |
| `tests/parity/str_high_then_high.json` | identical | exit 0 | exit 0 | `a597a719ed29` | `a597a719ed29` |
| `tests/parity/str_high_then_nonlow.json` | identical | exit 0 | exit 0 | `ed63c1d22fec` | `ed63c1d22fec` |
| `tests/parity/str_high_then_truncated.json` | identical | exit 0 | exit 0 | `d6f23335b73b` | `d6f23335b73b` |
| `tests/parity/str_in_array.json` | identical | exit 0 | exit 0 | `4ee4859f4539` | `4ee4859f4539` |
| `tests/parity/str_invalid_escape_a.json` | identical | exit 0 | exit 0 | `559a06c2b16b` | `559a06c2b16b` |
| `tests/parity/str_invalid_escape_quote_single.json` | identical | exit 0 | exit 0 | `cf7c26063e43` | `cf7c26063e43` |
| `tests/parity/str_invalid_escape_x.json` | identical | exit 0 | exit 0 | `654f06897e1f` | `654f06897e1f` |
| `tests/parity/str_invalid_utf8_continuation.json` | identical | exit 0 | exit 0 | `ccf4bcf5367b` | `ccf4bcf5367b` |
| `tests/parity/str_invalid_utf8_ff_fe.json` | identical | exit 0 | exit 0 | `85afa1d8b24c` | `85afa1d8b24c` |
| `tests/parity/str_invalid_utf8_overlong.json` | identical | exit 0 | exit 0 | `2a947de0e3f5` | `2a947de0e3f5` |
| `tests/parity/str_invalid_utf8_truncated.json` | identical | exit 0 | exit 0 | `5529484465e9` | `5529484465e9` |
| `tests/parity/str_key_raw_utf8.json` | identical | exit 0 | exit 0 | `cf249116ae4d` | `cf249116ae4d` |
| `tests/parity/str_key_unicode.json` | identical | exit 0 | exit 0 | `4f1599dc0b57` | `4f1599dc0b57` |
| `tests/parity/str_lone_high.json` | identical | exit 0 | exit 0 | `69168d6fc864` | `69168d6fc864` |
| `tests/parity/str_lone_high_then_char.json` | identical | exit 0 | exit 0 | `42d66089bcad` | `42d66089bcad` |
| `tests/parity/str_lone_low.json` | identical | exit 0 | exit 0 | `6056c8c82183` | `6056c8c82183` |
| `tests/parity/str_mixed_escapes_long.json` | identical | exit 0 | exit 0 | `415cc556d9e9` | `415cc556d9e9` |
| `tests/parity/str_nul_escape_then_text.json` | identical | exit 0 | exit 0 | `760af0e6516c` | `760af0e6516c` |
| `tests/parity/str_nul_inside.json` | identical | exit 0 | exit 0 | `02feb42bf7f3` | `02feb42bf7f3` |
| `tests/parity/str_quote_only.json` | identical | exit 0 | exit 0 | `cd7ec23e0291` | `cd7ec23e0291` |
| `tests/parity/str_raw_control_01.json` | identical | exit 0 | exit 0 | `b6d9dbed117c` | `b6d9dbed117c` |
| `tests/parity/str_raw_cr.json` | identical | exit 0 | exit 0 | `9f6de5c0a11d` | `9f6de5c0a11d` |
| `tests/parity/str_raw_del.json` | identical | exit 0 | exit 0 | `5fd3047c2ed0` | `5fd3047c2ed0` |
| `tests/parity/str_raw_newline.json` | identical | exit 0 | exit 0 | `a81dfb2a2218` | `a81dfb2a2218` |
| `tests/parity/str_raw_tab.json` | identical | exit 0 | exit 0 | `01106c7c40a2` | `01106c7c40a2` |
| `tests/parity/str_raw_utf8.json` | identical | exit 0 | exit 0 | `072f7606f147` | `072f7606f147` |
| `tests/parity/str_simple.json` | identical | exit 0 | exit 0 | `852315bf3d04` | `852315bf3d04` |
| `tests/parity/str_single_quoted.json` | identical | exit 0 | exit 0 | `e7d2e16242b9` | `e7d2e16242b9` |
| `tests/parity/str_slash_unescaped.json` | identical | exit 0 | exit 0 | `8ddec65275b9` | `8ddec65275b9` |
| `tests/parity/str_surrogate_max.json` | identical | exit 0 | exit 0 | `7d211bde0727` | `7d211bde0727` |
| `tests/parity/str_surrogate_pair.json` | identical | exit 0 | exit 0 | `349f7bff2f55` | `349f7bff2f55` |
| `tests/parity/str_surrogate_pair_upper.json` | identical | exit 0 | exit 0 | `c220a1f79384` | `c220a1f79384` |
| `tests/parity/str_truncated_u12.json` | identical | exit 0 | exit 0 | `e21cf990e637` | `e21cf990e637` |
| `tests/parity/str_truncated_u12_eof.json` | identical | exit 0 | exit 0 | `aafa35bb15b8` | `aafa35bb15b8` |
| `tests/parity/str_truncated_u_eof.json` | identical | exit 0 | exit 0 | `9a48f04c027d` | `9a48f04c027d` |
| `tests/parity/str_u0000.json` | identical | exit 0 | exit 0 | `6eac61480ccf` | `6eac61480ccf` |
| `tests/parity/str_u0000_only.json` | identical | exit 0 | exit 0 | `3fc7d853fcee` | `3fc7d853fcee` |
| `tests/parity/str_u0001.json` | identical | exit 0 | exit 0 | `fbcec2f5d1ff` | `fbcec2f5d1ff` |
| `tests/parity/str_u0080.json` | identical | exit 0 | exit 0 | `5b5fe7ed4c11` | `5b5fe7ed4c11` |
| `tests/parity/str_u00C9_upper.json` | identical | exit 0 | exit 0 | `5e33ba993a51` | `5e33ba993a51` |
| `tests/parity/str_u00e9.json` | identical | exit 0 | exit 0 | `5d41483924ae` | `5d41483924ae` |
| `tests/parity/str_u07ff.json` | identical | exit 0 | exit 0 | `ad534114943e` | `ad534114943e` |
| `tests/parity/str_u0800.json` | identical | exit 0 | exit 0 | `4d7b8f09db6f` | `4d7b8f09db6f` |
| `tests/parity/str_u20ac.json` | identical | exit 0 | exit 0 | `6ab82d3defb2` | `6ab82d3defb2` |
| `tests/parity/str_ud7ff.json` | identical | exit 0 | exit 0 | `8c0b91b8c66a` | `8c0b91b8c66a` |
| `tests/parity/str_ue000.json` | identical | exit 0 | exit 0 | `a3ded30d5f05` | `a3ded30d5f05` |
| `tests/parity/str_uffff.json` | identical | exit 0 | exit 0 | `ba80d84f5136` | `ba80d84f5136` |
| `tests/parity/str_unterminated.json` | identical | exit 0 | exit 0 | `8897a5cde873` | `8897a5cde873` |
| `tests/parity/str_unterminated_escape_quote.json` | identical | exit 0 | exit 0 | `2d1c92a4ef3a` | `2d1c92a4ef3a` |
| `tests/parity/str_utf8_surrogate_encoded.json` | identical | exit 0 | exit 0 | `941ba291b722` | `941ba291b722` |
| `tests/parity/trail_close_extra.json` | identical | exit 0 | exit 0 | `d84f116a2855` | `d84f116a2855` |
| `tests/parity/trail_comma.json` | identical | exit 0 | exit 0 | `f09c93f2303d` | `f09c93f2303d` |
| `tests/parity/trail_garbage.json` | identical | exit 0 | exit 0 | `66ff7cbae997` | `66ff7cbae997` |
| `tests/parity/trail_newline_garbage.json` | identical | exit 0 | exit 0 | `7a152f612334` | `7a152f612334` |
| `tests/parity/trail_nul_first.json` | identical | exit 0 | exit 0 | `c545308cf0cc` | `c545308cf0cc` |
| `tests/parity/trail_nul_garbage.json` | identical | exit 0 | exit 0 | `dbf387ef3f02` | `dbf387ef3f02` |
| `tests/parity/trail_nul_immediately.json` | identical | exit 0 | exit 0 | `3fc41937fc90` | `3fc41937fc90` |
| `tests/parity/trail_nul_ws_nul.json` | identical | exit 0 | exit 0 | `c2c06113c188` | `c2c06113c188` |
| `tests/parity/trail_number_nul_garbage.json` | identical | exit 0 | exit 0 | `50b6e92dcfa8` | `50b6e92dcfa8` |
| `tests/parity/trail_second_value.json` | identical | exit 0 | exit 0 | `02987f0c9ccc` | `02987f0c9ccc` |
| `tests/parity/trail_second_value_ws.json` | identical | exit 0 | exit 0 | `3d032d3a21fb` | `3d032d3a21fb` |
| `tests/parity/trail_string_nul_garbage.json` | identical | exit 0 | exit 0 | `26045cea64b0` | `26045cea64b0` |
| `tests/parity/trail_ws.json` | identical | exit 0 | exit 0 | `1a645178985c` | `1a645178985c` |
| `tests/parity/ws_crlf_doc.json` | identical | exit 0 | exit 0 | `9848da8e3a79` | `9848da8e3a79` |
| `tests/parity/ws_del.json` | identical | exit 0 | exit 0 | `e61406116916` | `e61406116916` |
| `tests/parity/ws_empty_file.json` | identical | exit 0 | exit 0 | `c68f887f825e` | `c68f887f825e` |
| `tests/parity/ws_formfeed.json` | identical | exit 0 | exit 0 | `d7816a819a26` | `d7816a819a26` |
| `tests/parity/ws_leading.json` | identical | exit 0 | exit 0 | `7b5d909cd411` | `7b5d909cd411` |
| `tests/parity/ws_nbsp.json` | identical | exit 0 | exit 0 | `bd0d2ed9fa49` | `bd0d2ed9fa49` |
| `tests/parity/ws_only.json` | identical | exit 0 | exit 0 | `e03558838e92` | `e03558838e92` |
| `tests/parity/ws_only_ctrl.json` | identical | exit 0 | exit 0 | `969eae271682` | `969eae271682` |
| `tests/parity/ws_only_nul.json` | identical | exit 0 | exit 0 | `b67f40b50476` | `b67f40b50476` |
| `tests/parity/ws_space.json` | identical | exit 0 | exit 0 | `fd9652b9202b` | `fd9652b9202b` |
| `tests/parity/ws_vtab.json` | identical | exit 0 | exit 0 | `5f5273ba2add` | `5f5273ba2add` |

## Divergences

None.

## Exceptions used

None.

## Gate problems

None.

## Pins

- 101 oracle files pinned
- 406 fixtures pinned

## Not covered

- Inputs outside the fixture globs; the gate proves parity only on the listed fixtures.
- stderr output (set `compare_stderr` to include it).
- Independence of the Rust driver: the gate only checks that it is not the same executable as the C driver.
