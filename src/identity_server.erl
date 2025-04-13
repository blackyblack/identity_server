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
    ets:new(vouches, [set, public, named_table]),
    ets:new(admins, [set, public, named_table]),
    ets:new(moderators, [set, public, named_table]),
    ets:new(id_proofs, [set, public, named_table]),
    setup_admins(),
    setup_moderators().

setup_admins() ->
    Admins = load_admins_or_moderators(admins),
    setup_admins(Admins).

setup_admins([]) -> [];

setup_admins([Admin | Rest]) when is_binary(Admin) ->
    admins:add_admin(Admin),
    setup_admins(Rest).

setup_moderators() ->
    Moders = load_admins_or_moderators(moderators),
    setup_moderators(Moders).

setup_moderators([]) -> [];

setup_moderators([Moder | Rest]) when is_binary(Moder) ->
    moderators:add_moderator_from_config(Moder),
    setup_moderators(Rest).

load_admins_or_moderators(AdminType) ->
    ReadFile = case AdminType of
        admins -> ?ADMINS_FILE;
        moderators -> ?MODERATORS_FILE
    end,
    case file:read_file(ReadFile) of
        {ok, FileContent} ->
            Admins = json:decode(FileContent),
            case Admins of
                [] -> [];
                [AdminsV] -> [AdminsV];
                _ -> {error, bad_format}
            end;
        {error, enoent} -> [];
        {error, Error} -> {error, Error}
    end.
