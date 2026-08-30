CC = gcc
CFLAGS = -Wall -Wextra -pthread
TARGET = sensor_client


SRCS = client.c process.c
OBJS = $(SRCS:.c=.o)


all: $(TARGET)

# Link the executable
$(TARGET): $(OBJS)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $(TARGET) $(OBJS)

# Compile C files to object files
%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

# Clean target for removing generated files
clean:
	rm -f $(OBJS) $(TARGET)