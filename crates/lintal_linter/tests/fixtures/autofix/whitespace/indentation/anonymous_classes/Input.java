import java.util.Comparator;
import java.util.function.Function;

public class Input {
    Runnable simpleAnonymous = new Runnable() {
        @Override
        public void run() {
          System.out.println("running");
        }
    };

    Comparator<String> comparatorAnonymous = new Comparator<String>() {
        @Override
        public int compare(String a, String b) {
             int lenA = a.length();
           int lenB = b.length();
            return Integer.compare(lenA, lenB);
        }
    };

    void methodWithAnonymous() {
        Thread t = new Thread(new Runnable() {
            @Override
            public void run() {
              for (int i = 0; i < 10; i++) {
                   System.out.println(i);
              }
            }
        });
        t.start();
    }

    void nestedAnonymous() {
        Function<String, Runnable> factory = new Function<String, Runnable>() {
            @Override
            public Runnable apply(String message) {
                return new Runnable() {
                    @Override
                    public void run() {
                      System.out.println(message);
                         if (message.isEmpty()) {
                           System.out.println("empty");
                         }
                    }
                };
            }
        };
    }

    Object complexAnonymous = new Object() {
        private int value = 0;

        public void increment() {
          value++;
            if (value > 10) {
             value = 0;
            }
        }

        public int getValue() {
           return value;
        }
    };
}
