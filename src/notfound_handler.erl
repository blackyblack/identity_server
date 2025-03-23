-module(notfound_handler).

-export([init/2]).

init(Req, Opts) ->
    Resp = cowboy_req:reply(404, #{<<"content-type">> => <<"application/json">>}, <<"{}">>, Req),
	{ok, Resp, Opts}.
