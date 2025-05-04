-module(identity_server).
-export([setup/0]).

-define(DEFAULT_PORT, 8000).
-define(ADMINS_FILE, "admins.json").
-define(MODERATORS_FILE, "moderators.json").
-define(GENESIS_FILE, "genesis.json").

setup() ->
    dotenv:init(),
    ets:new(identity_nonce_consumed, [set, public, named_table]),
    ets:new(proof_nonce_consumed, [set, public, named_table]),
    ets:new(moderators_nonce_consumed, [set, public, named_table]),
    ets:new(admins_nonce_consumed, [set, public, named_table]),
    ets:new(vouches, [set, public, named_table]),
    ets:new(admins, [set, public, named_table]),
    ets:new(moderators, [set, public, named_table]),
    ets:new(id_proofs, [set, public, named_table]),
    ets:new(penalties, [set, public, named_table]),
    setup_admins(),
    setup_moderators(),
    setup_genesis().

setup_admins() ->
    Admins = load_json(?ADMINS_FILE),
    lists:foreach(fun(A) -> admins:add_admin_from_config(A) end, Admins).

setup_moderators() ->
    Moders = load_json(?MODERATORS_FILE),
    lists:foreach(fun(M) -> moderators:add_moderator_from_config(M) end, Moders).

setup_genesis() ->
    GenesisUsers = load_json(?GENESIS_FILE),
    lists:foreach(fun(#{<<"user">> := User, <<"idt">> := Balance}) -> identity:set_idt_by_config(User, Balance) end, GenesisUsers).

load_json(FileName) ->
    case file:read_file(FileName) of
        {ok, FileContent} -> json:decode(FileContent);
        {error, enoent} -> [];
        {error, Error} -> {error, Error}
    end.
