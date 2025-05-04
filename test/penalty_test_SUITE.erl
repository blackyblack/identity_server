-module(penalty_test_SUITE).
-compile([export_all, nowarn_export_all]).

-include_lib("common_test/include/ct.hrl").

-define(ADMIN, <<"admin">>).
-define(MODERATOR, <<"moderator">>).
-define(PROOF_ID, <<"id1">>).
-define(PUNISH_ID, <<"id2">>).

-define(USER_A, <<"userA">>).

all() -> [
    basic_test,
    vouchee_punish_test,
    max_punish_test
].

init_per_suite(Config) ->
    application:start(identity_server),
    admins:add_admin_from_config(?ADMIN),
    moderators:add_moderator(?ADMIN, ?MODERATOR),
    Config.

end_per_suite(Config) ->
    application:stop(identity_server),
    Config.

init_per_testcase(_TestCase, Config) ->
    ets:delete_all_objects(vouches),
    ets:delete_all_objects(id_proofs),
    ets:delete_all_objects(penalties),
    Config.

basic_test(_Config) ->
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 5, ?PROOF_ID),
    5 = identity:idt(?USER_A),
    {ok, _} = identity:punish(?MODERATOR, ?USER_A, 4, ?PUNISH_ID),   
    1 = identity:idt(?USER_A),
    ok.

vouchee_punish_test(_Config) ->
    Voucher1 = <<"userB">>,
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 50, ?PROOF_ID),
    50 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(?USER_A, Voucher1),
    5 = identity:idt(Voucher1),
    {ok, _} = identity:punish(?MODERATOR, Voucher1, 10, ?PUNISH_ID),
    0 = identity:idt(Voucher1),
    49 = identity:idt(?USER_A),
    ok.

max_punish_test(_Config) ->
    Voucher1 = <<"userB">>,
    ok = identity:set_idt_by_proof(?MODERATOR, ?USER_A, 50000, ?PROOF_ID),
    50000 = identity:idt(?USER_A),
    {ok, _} = identity:vouch(?USER_A, Voucher1),
    5000 = identity:idt(Voucher1),
    {ok, _} = identity:punish(?MODERATOR, Voucher1, 1, ?PUNISH_ID),
    4999 = identity:idt(Voucher1),
    {ok, _} = identity:punish(?MODERATOR, Voucher1, 1, <<"id3">>),
    4998 = identity:idt(Voucher1),
    50000 = identity:idt(?USER_A),
    {ok, _} = identity:punish(?MODERATOR, Voucher1, 10000, <<"id4">>),
    0 = identity:idt(Voucher1),
    49000 = identity:idt(?USER_A),
    {ok, _} = identity:punish(?MODERATOR, Voucher1, 100000, <<"id5">>),
    0 = identity:idt(Voucher1),
    39000 = identity:idt(?USER_A),
    {ok, _} = identity:punish(?MODERATOR, Voucher1, 100000, <<"id6">>),
    0 = identity:idt(Voucher1),
    % maximum penalty from a single vouchee is 20000 IDT
    30000 = identity:idt(?USER_A),
    ok.
