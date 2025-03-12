-module(identity_server_sup).
-behaviour(supervisor).
-export([start_link/1, init/1]).

-include("./include/args.hrl").

-define(SERVER, ?MODULE).

start_link(Args) ->
    supervisor:start_link({local, ?SERVER}, ?MODULE, Args).

init(#args{port = Port}) ->
    SupFlags = #{
        strategy => one_for_one
    },
    ElliOpts = [
        {callback, http_handler},
        {port, Port}
    ],
    ElliSpec = {
        _Id = elli_minimal_http,
        _Start = {elli, start_link, [ElliOpts]},
        _Restart = permanent,
        _Shutdown = 5000,
        _Worker = worker,
        _Modules = [elli]},
    ChildSpecs = [ElliSpec],
    {ok, {SupFlags, ChildSpecs}}.
