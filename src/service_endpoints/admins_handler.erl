-module(admins_handler).

-export([init/2]).

init(#{method := <<"GET">>} = Req, Opts) ->
    Admins = admins:admins_list(),
    AdminResp = json:encode(Admins),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, AdminResp, Req),
    {ok, Resp, Opts}.
