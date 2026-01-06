import java.util.List;
import java.util.ArrayList;
import java.util.stream.Stream;
import java.util.function.Consumer;

public class Input {
    List<String> items = new ArrayList<>();

    void simpleLambda() {
        items.forEach(item -> {
            System.out.println(item);
        });
    }

    void nestedLambdas() {
        Stream.of("a", "b", "c")
            .map(s -> {
                return s.toUpperCase();
            })
            .filter(s -> {
                return s.startsWith("A");
            })
            .forEach(s -> {
                System.out.println(s);
            });
    }

    void lambdaWithMultipleStatements() {
        Consumer<String> processor = item -> {
            String upper = item.toUpperCase();
            String trimmed = upper.trim();
            System.out.println(trimmed);
        };
    }

    void deeplyNestedLambda() {
        items.stream()
            .map(item -> {
                return item.chars()
                    .mapToObj(c -> {
                        return String.valueOf((char) c);
                    })
                    .reduce("", (a, b) -> {
                        return a + b;
                    });
            })
            .forEach(System.out::println);
    }

    Runnable createRunnable() {
        return () -> {
            int x = 1;
            int y = 2;
            int z = x + y;
            System.out.println(z);
        };
    }
}
