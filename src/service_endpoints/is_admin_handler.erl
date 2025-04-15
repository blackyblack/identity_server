-module(is_admin_handler).

-export([init/2]).

init(#{method := <<"GET">>, bindings := #{user := User}} = Req, Opts) ->
    AdminResp = json:encode(#{<<"is_admin">> => admins:is_admin(User)}),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, AdminResp, Req),
    {ok, Resp, Opts}.
