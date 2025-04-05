-module(dotenv).

-export([init/0]).

-spec init() -> ok | {error, any()}.
init() ->
    maybe
        {ok, Config} ?= dotenv_reader:read(<<".env">>),
        maps:foreach(
            fun(K, V) ->
                os:putenv(binary_to_list(K), binary_to_list(V))
            end,
            Config),
        ok
    end.
