-module(vouch_test_SUITE).
-compile([export_all, nowarn_export_all]).

-include_lib("common_test/include/ct.hrl").

-define(ADMIN, <<"admin">>).
-define(MODERATOR, <<"moderator">>).
-define(PROOF_ID, <<"id1">>).

-define(USER_A, <<"userA">>).

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
    admins:add_admin(?ADMIN),
    moderators:add_moderator(?ADMIN, ?MODERATOR),
    Config.

end_per_suite(Config) ->
    application:stop(identity_server),
    Config.

init_per_testcase(_TestCase, Config) ->
    ets:delete_all_objects(vouches),
    ets:delete_all_objects(id_proofs),
    Config.

basic_test(_Config) ->
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 5, ?PROOF_ID),
    5 = identity:idt(?USER_A),
    % vouch for myself does not change balance
    {ok, _} = identity:vouch(?USER_A, ?USER_A),   
    5 = identity:idt(?USER_A),
    % does not allow to set too big IDT balance by proof
    {error, _} = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 50001, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 50, ?PROOF_ID),
    50 = identity:idt(?USER_A),
    ok.

basic_vouch_test(_Config) ->
    Voucher = <<"userB">>,
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher, 50, ?PROOF_ID),
    {ok, _} = identity:vouch(Voucher, ?USER_A),
    5 = identity:idt(?USER_A),
    ok.

two_layers_vouch_test(_Config) ->
    VoucherLayer1 = <<"userB">>,
    VoucherLayer2 = <<"userC">>,
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 10, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, VoucherLayer1, 10, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, VoucherLayer2, 500, ?PROOF_ID),
    {ok, _} = identity:vouch(VoucherLayer2, VoucherLayer1),
    % 500 * 0.1 + 10
    60 = identity:idt(VoucherLayer1),
    {ok, _} = identity:vouch(VoucherLayer1, ?USER_A),
    % 60 * 0.1 + 10
    16 = identity:idt(?USER_A),
    ok.

top5_vouch_test(_Config) ->
    Voucher1 = <<"userB">>,
    Voucher2 = <<"userC">>,
    Voucher3 = <<"userD">>,
    Voucher4 = <<"userE">>,
    Voucher5 = <<"userF">>,
    Voucher6 = <<"userG">>,
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 10, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher1, 10, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher2, 20, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher3, 30, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher4, 40, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher5, 50, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher6, 60, ?PROOF_ID),
    {ok, _} = identity:vouch(Voucher1, ?USER_A),
    % 10 * 0.1 + 10
    11 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(Voucher2, ?USER_A),
    % 20 * 0.1 + 10 * 0.1 + 10
    13 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(Voucher2, ?USER_A),
    % 20 * 0.1 + 10 * 0.1 + 10
    13 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(Voucher3, ?USER_A),
    % 30 * 0.1 + 20 * 0.1 + 10 * 0.1 + 10
    16 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(Voucher4, ?USER_A),
    % 40 * 0.1 + 30 * 0.1 + 20 * 0.1 + 10 * 0.1 + 10
    20 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(Voucher5, ?USER_A),
    % 50 * 0.1 + 40 * 0.1 + 30 * 0.1 + 20 * 0.1 + 10 * 0.1 + 10
    25 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(Voucher6, ?USER_A),
    % 60 * 0.1 + 50 * 0.1 + 40 * 0.1 + 30 * 0.1 + 20 * 0.1 + 10
    30 = identity:idt(?USER_A),
    ok.

cyclic_vouch_test(_Config) ->
    Voucher1 = <<"userB">>,
    Voucher2 = <<"userC">>,
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 100, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher1, 100, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher2, 200, ?PROOF_ID),
    {ok, _} = identity:vouch(Voucher1, Voucher2),
    % 100 * 0.1 + 200
    210 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher2, ?USER_A),
    % 210 * 0.1 + 100
    121 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(?USER_A, Voucher1),
    % 121 * 0.1 + 100
    112 = identity:idt(Voucher1),
    % test that cyclic dependency does not break balance calculation
    211 = identity:idt(Voucher2),
    121 = identity:idt(?USER_A),
    % test that additional vouches do not increase previous balances
    {ok, _} = identity:vouch(Voucher1, Voucher2),
    211 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher2, ?USER_A),
    121 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(?USER_A, Voucher1),
    112 = identity:idt(Voucher1),
    ok.

cyclic_mutual_vouch_test(_Config) ->
    Voucher1 = <<"userB">>,
    Voucher2 = <<"userC">>,
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 100, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher1, 100, ?PROOF_ID),
    ok = identity:set_idt_by_proof(?MODERATOR, Voucher2, 200, ?PROOF_ID),
    {ok, _} = identity:vouch(Voucher1, Voucher2),
    % 100 * 0.1 + 200
    210 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher2, ?USER_A),
    % 210 * 0.1 + 100
    121 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(?USER_A, Voucher1),
    % 121 * 0.1 + 100
    % balances after cyclic dependencies
    121 = identity:idt(?USER_A),
    112 = identity:idt(Voucher1),
    211 = identity:idt(Voucher2),
    % test mutual vouches
    {ok, _} = identity:vouch(Voucher2, Voucher1),
    132 = identity:idt(Voucher1),
    {ok, _} = identity:vouch(?USER_A, Voucher2),
    220 = identity:idt(Voucher2),
    {ok, _} = identity:vouch(Voucher1, ?USER_A),
    131 = identity:idt(?USER_A),
    % balances after mutual dependencies
    131 = identity:idt(?USER_A),
    132 = identity:idt(Voucher1),
    221 = identity:idt(Voucher2),
    ok.
