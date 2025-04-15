-module(identity_signature).

-export([
    is_nonce_consumed/2,
    verify_and_consume_signature/5,
    vouch_signature_message/2,
    proof_signature_message/4,
    moderators_signature_message/2,
    admins_signature_message/2
]).

-spec is_nonce_consumed(Nonces :: [integer()], integer()) -> boolean().
is_nonce_consumed([], _Nonce) -> false;
is_nonce_consumed([{_User, LastNonce}], Nonce) -> Nonce =< LastNonce;
% only one nonce should be in storage, fail on multiple records
is_nonce_consumed(_Nonces, _Nonce) -> true.

-spec verify_and_consume_signature(Action, SignatureEncoded, PublicKeyEncoded, Nonce, Message) -> ok | Error when
    Action :: vouch | proof | moderators | admins,
    SignatureEncoded :: binary(),
    PublicKeyEncoded :: binary(),
    Nonce :: integer(),
    Message :: string(),
    Error :: {error, nonce_consumed} | {error, bad_signature}.
verify_and_consume_signature(Action, SignatureEncoded, PublicKeyEncoded, Nonce, Message) ->
    Table = case Action of
        vouch -> identity_nonce_consumed;
        proof -> proof_nonce_consumed;
        moderators -> moderators_nonce_consumed;
        admins -> admins_nonce_consumed
    end,
    maybe
        ok ?= case identity_signature:is_nonce_consumed(ets:lookup(Table, PublicKeyEncoded), Nonce) of
            false -> ok;
            true -> {error, nonce_consumed}
        end,
        Signature = base64:decode(SignatureEncoded),
        PublicKey = base58:base58_to_binary(binary_to_list(PublicKeyEncoded)),
        ok ?= case crypto:verify(eddsa, none, Message, Signature, [PublicKey, ed25519]) of
            true -> ok;
            false -> {error, bad_signature}
        end,
        ets:insert(Table, {PublicKeyEncoded, Nonce}),
        ok
    end.

-spec vouch_signature_message(UserEncoded :: binary(), Nonce :: integer()) -> binary().
vouch_signature_message(UserEncoded, Nonce) ->
    list_to_binary(string:join(["vouch", binary_to_list(UserEncoded), integer_to_list(Nonce)], "/")).

-spec proof_signature_message(UserEncoded :: binary(), Nonce :: integer(), Balance :: integer(), ProofId :: binary()) -> binary().
proof_signature_message(UserEncoded, Nonce, Balance, ProofId) ->
    list_to_binary(string:join([
        "proof",
        binary_to_list(UserEncoded),
        integer_to_list(Nonce),
        integer_to_list(Balance),
        binary_to_list(ProofId)
    ], "/")).

-spec moderators_signature_message(UserEncoded :: binary(), Nonce :: integer()) -> binary().
moderators_signature_message(UserEncoded, Nonce) ->
    list_to_binary(string:join(["moderators", binary_to_list(UserEncoded), integer_to_list(Nonce)], "/")).

-spec admins_signature_message(UserEncoded :: binary(), Nonce :: integer()) -> binary().
admins_signature_message(UserEncoded, Nonce) ->
    list_to_binary(string:join(["admins", binary_to_list(UserEncoded), integer_to_list(Nonce)], "/")).
