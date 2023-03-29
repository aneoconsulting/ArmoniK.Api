from grpc import Channel


class DummyChannel(Channel):
    def __init__(self):
        self.method_dict = {}

    def stream_unary(self, *args, **kwargs):
        return self.get_method(args[0])

    def unary_stream(self, *args, **kwargs):
        return self.get_method(args[0])

    def unary_unary(self, *args, **kwargs):
        return self.get_method(args[0])

    def stream_stream(self, *args, **kwargs):
        return self.get_method(args[0])

    def set_instance(self, instance):
        self.method_dict = {func: getattr(instance, func) for func in dir(type(instance)) if callable(getattr(type(instance), func)) and not func.startswith("__")}

    def get_method(self, name: str):
        return self.method_dict.get(name.split("/")[-1], None)

    def subscribe(self, callback, try_to_connect=False):
        pass

    def unsubscribe(self, callback):
        pass

    def close(self):
        pass

    def __enter__(self):
        pass

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass

