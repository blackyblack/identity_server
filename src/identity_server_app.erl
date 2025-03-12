-module(identity_server_app).
-behaviour(application).
-export([start/2, stop/1]).

-include("./include/args.hrl").

-define(DEFAULT_PORT, 8000).

start(_StartType, _StartArgs) ->
    ok = dotenv:init(),
    Port = case os:getenv("PORT") of
        false -> ?DEFAULT_PORT;
        P -> list_to_integer(P)
    end,
    Ret = identity_server_sup:start_link(#args{port = Port}),
    app_logger:info("Identity server started at localhost:~p", [Port]),
    Ret.

stop(_State) ->
    ok.
