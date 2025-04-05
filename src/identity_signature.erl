-module(identity_signature).

-export([is_nonce_consumed/2, verify_and_consume_signature/4, message_for_signature/3]).

-spec is_nonce_consumed(Nonces :: [integer()], integer()) -> boolean().
is_nonce_consumed([], _Nonce) -> false;
is_nonce_consumed([{_User, LastNonce}], Nonce) -> Nonce =< LastNonce;
% only one nonce should be in storage, fail on multiple records
is_nonce_consumed(_Nonces, _Nonce) -> true.

-spec verify_and_consume_signature(SignatureEncoded, PublicKeyEncoded, Nonce, Message) -> ok | Error when
      SignatureEncoded :: binary(),
      PublicKeyEncoded :: binary(),
      Nonce :: integer(),
      Message :: string(),
      Error :: {error, nonce_consumed} | {error, bad_signature}.
verify_and_consume_signature(SignatureEncoded, PublicKeyEncoded, Nonce, Message) ->
    maybe
        ok ?= case identity_signature:is_nonce_consumed(ets:lookup(identity_nonce_consumed, PublicKeyEncoded), Nonce) of
            false -> ok;
            true -> {error, nonce_consumed}
        end,
        Signature = base64:decode(SignatureEncoded),
        PublicKey = base58:base58_to_binary(binary_to_list(PublicKeyEncoded)),
        ok ?= case crypto:verify(eddsa, none, Message, Signature, [PublicKey, ed25519]) of
            true -> ok;
            false -> {error, bad_signature}
        end,
        ets:insert(identity_nonce_consumed, {PublicKeyEncoded, Nonce}),
        ok
    end.

-spec message_for_signature(Type :: term(), SignerEncoded :: binary(), Nonce :: integer()) -> binary().
message_for_signature(vouch, SignerEncoded, Nonce) ->
    list_to_binary(string:join(["vouch", binary_to_list(SignerEncoded), integer_to_list(Nonce)], "/")).
