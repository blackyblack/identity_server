-module(admins).

-export([is_admin/1, add_admin/1, remove_admin/1]).

is_admin(User) ->
    ets:member(admins, User).

add_admin(User) ->
    ets:insert(admins, {User}).

remove_admin(User) ->
    ets:delete_object(admins, {User}).
