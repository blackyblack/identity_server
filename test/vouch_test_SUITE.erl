-module(vouch_test_SUITE).
-compile([export_all, nowarn_export_all]).

-include_lib("common_test/include/ct.hrl").

all() -> [
    basic_test,
    basic_vouch_test,
    two_layers_vouch_test,
    top5_vouch_test,
    cyclic_vouch_test,
    cyclic_mutual_vouch_test
].

init_per_suite(Config) ->
    application:start(identity_server),
    moderators:add_moderator(<<"moderator">>),
    Config.

end_per_suite(Config) ->
    application:stop(identity_server),
    Config.

init_per_testcase(_TestCase, Config) ->
    ets:delete_all_objects(vouches),
    ets:delete_all_objects(id_proofs),
    Config.

basic_test(_Config) ->
    To = <<"userA">>,
    Moderator = <<"moderator">>,
    ok = identity:set_idt_by_proof(Moderator, To, 5),
    5 = identity:idt(To),
    % vouch for myself does not change balance
    {ok, _} = identity:vouch(To, To),   
    5 = identity:idt(To),
    % does not allow to set too big IDT balance by proof
    {error, _} = identity:set_idt_by_proof(Moderator, To, 50001),
    ok = identity:set_idt_by_proof(Moderator, To, 50),
    50 = identity:idt(To),
    ok.

basic_vouch_test(_Config) ->
    To = <<"userA">>,
    Voucher = <<"userB">>,
    Moderator = <<"moderator">>,
    ok = identity:set_idt_by_proof(Moderator, Voucher, 50),
    {ok, _} = identity:vouch(Voucher, To),
    5 = identity:idt(To),
    ok.

two_layers_vouch_test(_Config) ->
    To = <<"userA">>,
    VoucherLayer1 = <<"userB">>,
    VoucherLayer2 = <<"userC">>,
    Moderator = <<"moderator">>,
    ok = identity:set_idt_by_proof(Moderator, To, 10),
    ok = identity:set_idt_by_proof(Moderator, VoucherLayer1, 10),
    ok = identity:set_idt_by_proof(Moderator, VoucherLayer2, 500),
    {ok, _} = identity:vouch(VoucherLayer2, VoucherLayer1),
    % 500 * 0.1 + 10
    60 = identity:idt(VoucherLayer1),
    {ok, _} = identity:vouch(VoucherLayer1, To),
    % 60 * 0.1 + 10
    16 = identity:idt(To),
    ok.

top5_vouch_test(_Config) ->
    To = <<"userA">>,
    Voucher1 = <<"userB">>,
    Voucher2 = <<"userC">>,
    Voucher3 = <<"userD">>,
    Voucher4 = <<"userE">>,
    Voucher5 = <<"userF">>,
    Voucher6 = <<"userG">>,
    Moderator = <<"moderator">>,
    ok = identity:set_idt_by_proof(Moderator, To, 10),
    ok = identity:set_idt_by_proof(Moderator, Voucher1, 10),
    ok = identity:set_idt_by_proof(Moderator, Voucher2, 20),
    ok = identity:set_idt_by_proof(Moderator, Voucher3, 30),
    ok = identity:set_idt_by_proof(Moderator, Voucher4, 40),
    ok = identity:set_idt_by_proof(Moderator, Voucher5, 50),
    ok = identity:set_idt_by_proof(Moderator, Voucher6, 60),
    {ok, _} = identity:vouch(Voucher1, To),
    % 10 * 0.1 + 10
    11 = identity:idt(To),
    {ok, _} = identity:vouch(Voucher2, To),
    % 20 * 0.1 + 10 * 0.1 + 10
    13 = identity:idt(To),
    {ok, _} = identity:vouch(Voucher2, To),
    % 20 * 0.1 + 10 * 0.1 + 10
    13 = identity:idt(To),
    {ok, _} = identity:vouch(Voucher3, To),
    % 30 * 0.1 + 20 * 0.1 + 10 * 0.1 + 10
    16 = identity:idt(To),
    {ok, _} = identity:vouch(Voucher4, To),
    % 40 * 0.1 + 30 * 0.1 + 20 * 0.1 + 10 * 0.1 + 10
    20 = identity:idt(To),
    {ok, _} = identity:vouch(Voucher5, To),
    % 50 * 0.1 + 40 * 0.1 + 30 * 0.1 + 20 * 0.1 + 10 * 0.1 + 10
    25 = identity:idt(To),
    {ok, _} = identity:vouch(Voucher6, To),
    % 60 * 0.1 + 50 * 0.1 + 40 * 0.1 + 30 * 0.1 + 20 * 0.1 + 10
    30 = identity:idt(To),
    ok.

cyclic_vouch_test(_Config) ->
    To = <<"userA">>,
    Voucher1 = <<"userB">>,
    Voucher2 = <<"userC">>,
    Moderator = <<"moderator">>,
    ok = identity:set_idt_by_proof(Moderator, To, 100),
    ok = identity:set_idt_by_proof(Moderator, Voucher1, 100),
    ok = identity:set_idt_by_proof(Moderator, Voucher2, 200),
    {ok, _} = identity:vouch(Voucher1, Voucher2),
    % 100 * 0.1 + 200
    210 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher2, To),
    % 210 * 0.1 + 100
    121 = identity:idt(To),
    {ok, _} = identity:vouch(To, Voucher1),
    % 121 * 0.1 + 100
    112 = identity:idt(Voucher1),
    % test that cyclic dependency does not break balance calculation
    211 = identity:idt(Voucher2),
    121 = identity:idt(To),
    % test that additional vouches do not increase previous balances
    {ok, _} = identity:vouch(Voucher1, Voucher2),
    211 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher2, To),
    121 = identity:idt(To),
    {ok, _} = identity:vouch(To, Voucher1),
    112 = identity:idt(Voucher1),
    ok.

cyclic_mutual_vouch_test(_Config) ->
    To = <<"userA">>,
    Voucher1 = <<"userB">>,
    Voucher2 = <<"userC">>,
    Moderator = <<"moderator">>,
    ok = identity:set_idt_by_proof(Moderator, To, 100),
    ok = identity:set_idt_by_proof(Moderator, Voucher1, 100),
    ok = identity:set_idt_by_proof(Moderator, Voucher2, 200),
    {ok, _} = identity:vouch(Voucher1, Voucher2),
    % 100 * 0.1 + 200
    210 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher2, To),
    % 210 * 0.1 + 100
    121 = identity:idt(To),
    {ok, _} = identity:vouch(To, Voucher1),
    % 121 * 0.1 + 100
    % balances after cyclic dependencies
    121 = identity:idt(To),
    112 = identity:idt(Voucher1),
    211 = identity:idt(Voucher2),
    % test mutual vouches
    {ok, _} = identity:vouch(Voucher2, Voucher1),
    132 = identity:idt(Voucher1),
    {ok, _} = identity:vouch(To, Voucher2),
    220 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher1, To),
    131 = identity:idt(To),
    % balances after mutual dependencies
    131 = identity:idt(To),
    132 = identity:idt(Voucher1),
    221 = identity:idt(Voucher2),
    ok.
