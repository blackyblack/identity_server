-module(http_handler).
-behaviour(elli_handler).
-export([handle/2, handle_event/3]).

-include_lib("elli/include/elli.hrl").

handle(#req { method = 'GET', path = [<<"idt">>], args = [{<<"user">>, User}] }, _Args) ->
    Response = iolist_to_binary(io_lib:format("Hello ~s", [User])),
    {ok, [], Response};

handle(_Req, _Args) ->
    {404, [], <<"Not Found">>}.

handle_event(_Event, _Data, _Args) ->
    ok.