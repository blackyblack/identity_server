-module(identity_server_app).
-behaviour(application).
-export([start/2, stop/1]).

-include("./include/args.hrl").

-define(DEFAULT_PORT, 8000).

start(_StartType, _StartArgs) ->
    dotenv:init(),
    {ok, _} = application:ensure_all_started(cowboy),
    Port = case os:getenv("PORT") of
        false -> ?DEFAULT_PORT;
        P -> list_to_integer(P)
    end,
    Dispatch = cowboy_router:compile([
		{'_', [
			{"/vouch/:user", vouch_handler, []},
            {'_', notfound_handler, []}
		]}
	]),
	{ok, Pid} = cowboy:start_clear(http, [{port, Port}], #{
		env => #{dispatch => Dispatch}
	}),
    ets:new(identity_nonce_consumed, [set, public, named_table]),
    ets:new(vouches, [set, public, named_table]),
    app_logger:info("Identity server started at localhost:~p", [Port]),
    {ok, Pid}.

stop(_State) ->
    ok = cowboy:stop_listener(http).
