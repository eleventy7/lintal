import java.util.function.Consumer;

public class Input {
    Consumer<String> empty = s -> {};
    Consumer<String> nonEmpty = s -> {  System.out.println(s);  };
}
