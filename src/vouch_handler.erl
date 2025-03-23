-module(vouch_handler).

-export([init/2]).

init(#{method := <<"POST">>, bindings := #{user := UserEncoded}} = Req, Opts) ->
    {ok, JsonData, State} = cowboy_req:read_body(Req),
    #{<<"signature">> := SignatureEncoded, <<"nonce">> := Nonce, <<"signer">> := PublicKeyEncoded} = json:decode(JsonData),
    % nonce should not be consumed
    [] = ets:match(identity_nonce_consumed, {{PublicKeyEncoded, Nonce}}),
    Signature = base64:decode(SignatureEncoded),
    PublicKey = base58:base58_to_binary(binary_to_list(PublicKeyEncoded)),
    User = base58:base58_to_binary(binary_to_list(UserEncoded)),
    RefMessage = list_to_binary(string:join(["vouch", binary_to_list(UserEncoded), integer_to_list(Nonce)], "/")),
    true = crypto:verify(eddsa, none, RefMessage, Signature, [PublicKey, ed25519]),
    true = ets:insert(identity_nonce_consumed, {{PublicKeyEncoded, Nonce}}),
    {ok, VouchResp} = vouch(PublicKey, User),
    Resp = cowboy_req:reply(200, #{<<"content-type">> => <<"application/json">>}, VouchResp, State),
    {ok, Resp, Opts}.
    % TODO: put signer's vouch to dets

vouch(From, To) ->
    % TODO: calculate user 'To' new IDT
    Idt = 0,
    FromStr = list_to_binary(base58:binary_to_base58(From)),
    ToStr = list_to_binary(base58:binary_to_base58(To)),
    {ok, json:encode(#{<<"from">> => FromStr, <<"to">> => ToStr, <<"idt">> => Idt})}.
